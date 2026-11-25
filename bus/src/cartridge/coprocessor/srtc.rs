//! S-RTC: Sharp real-time clock of Daikaijuu Monogatari II
//!
//! The chip keeps the date and time as 13 4-bit digits and talks to the
//! CPU through two registers in banks `$00-$3F` / `$80-$BF`:
//!
//! - `$2800` (read): next digit of the clock, framed by `$0F` markers
//! - `$2801` (write): command or digit (only the low 4 bits are used)
//!
//! Digits, in transfer order:
//!
//! | Index | Digit                | Index | Digit                      |
//! |-------|----------------------|-------|----------------------------|
//! | 0     | seconds, ones        | 7     | day, tens                  |
//! | 1     | seconds, tens        | 8     | month (1-12)               |
//! | 2     | minutes, ones        | 9     | year, ones                 |
//! | 3     | minutes, tens        | 10    | year, tens                 |
//! | 4     | hours, ones          | 11    | year, hundreds (since 1000)|
//! | 5     | hours, tens          | 12    | day of week (0 = Sunday)   |
//! | 6     | day, ones            |       |                            |
//!
//! Writing `$0D` starts a read: the next reads of `$2800` return `$0F`, the
//! 13 digits, then `$0F` again, and so on. Writing `$0E` starts a command:
//! `$0` then sets the clock from the next 12 digits (the day of week is
//! computed by the chip), and `$4` clears it.
//!
//! On the board the chip runs from its own crystal and battery. Here it
//! counts emulated master cycles, so the clock follows the emulation speed
//! and is deterministic. It is not saved between sessions yet.

use super::{CartridgeMemory, CoprocessorChip};
use crate::bus::AccessSpeed;
use common::snes_address::SnesAddress;

/// Address of the read register
const READ_REG: u16 = 0x2800;
/// Address of the write register
const WRITE_REG: u16 = 0x2801;
/// Number of digits of the clock
const DIGITS: usize = 13;
/// Marker returned before and after the digits
const FRAME: u8 = 0x0F;

/// Master clock frequency (NTSC, 21.477272... MHz) is `MASTER_HZ_NUM / MASTER_HZ_DEN`
///
/// The only S-RTC game is Japanese, so the NTSC clock is enough.
/// Kept as a fraction so a second lasts exactly the right number of cycles on average.
const MASTER_HZ_NUM: u64 = 236_250_000;
/// See `MASTER_HZ_NUM`
const MASTER_HZ_DEN: u64 = 11;

/// What the next access to the chip does
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Waiting for `$0D` (read) or `$0E` (command)
    Ready,
    /// `$0E` was written, waiting for the command number
    Command,
    /// Reads of `$2800` return the clock digits
    Read,
    /// Writes of `$2801` set the clock digits
    Write,
}

/// Date and time held by the chip, decoded from its digits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DateTime {
    second: u32,
    minute: u32,
    hour: u32,
    day: u32,
    month: u32,
    year: u32,
}

impl DateTime {
    fn from_digits(d: &[u8; DIGITS]) -> Self {
        let d = d.map(u32::from);
        Self {
            second: d[0] + d[1] * 10,
            minute: d[2] + d[3] * 10,
            hour: d[4] + d[5] * 10,
            day: d[6] + d[7] * 10,
            month: d[8],
            year: 1000 + d[9] + d[10] * 10 + d[11] * 100,
        }
    }

    fn to_digits(self) -> [u8; DIGITS] {
        let since_1000 = self.year.saturating_sub(1000);
        // every value fits in 4 bits once split in digits
        [
            self.second % 10,
            self.second / 10,
            self.minute % 10,
            self.minute / 10,
            self.hour % 10,
            self.hour / 10,
            self.day % 10,
            self.day / 10,
            self.month,
            since_1000 % 10,
            since_1000 / 10 % 10,
            since_1000 / 100 % 16,
            weekday(self.year, self.month, self.day),
        ]
        .map(|digit| (digit & 0x0F) as u8)
    }

    /// Adds one second, carrying into the minutes, hours, days...
    ///
    /// A value the game set out of range (e.g. 75 seconds) wraps on the next
    /// carry instead of counting up forever.
    fn tick(&mut self) {
        self.second += 1;
        if self.second < 60 {
            return;
        }
        self.second = 0;
        self.minute += 1;
        if self.minute < 60 {
            return;
        }
        self.minute = 0;
        self.hour += 1;
        if self.hour < 24 {
            return;
        }
        self.hour = 0;
        self.day += 1;
        if self.day <= days_in_month(self.year, self.month) {
            return;
        }
        self.day = 1;
        self.month += 1;
        if self.month <= 12 {
            return;
        }
        self.month = 1;
        self.year += 1;
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Number of days in `month`, 31 for an invalid month
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 31,
    }
}

/// Day of the week (0 = Sunday) of a date, with Sakamoto's method
fn weekday(year: u32, month: u32, day: u32) -> u32 {
    const OFFSETS: [u32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let month = month.clamp(1, 12);
    let year = if month < 3 { year - 1 } else { year };
    (year + year / 4 - year / 100 + year / 400 + OFFSETS[month as usize - 1] + day) % 7
}

/// S-RTC chip
#[derive(Debug)]
pub struct Srtc {
    /// Clock digits, see the module documentation
    digits: [u8; DIGITS],
    mode: Mode,
    /// Next digit to transfer, `None` before the `$0F` start marker
    index: Option<usize>,
    /// Master cycles counted toward the next second, times `MASTER_HZ_DEN`
    subsecond: u64,
}

impl Default for Srtc {
    fn default() -> Self {
        Self::new()
    }
}

impl Srtc {
    /// Creates the chip, with a cleared clock (as with a fresh battery)
    pub fn new() -> Self {
        Self {
            digits: [0; DIGITS],
            mode: Mode::Read,
            index: None,
            subsecond: 0,
        }
    }

    /// Whether `addr` is in the banks the chip registers are mapped in
    fn in_banks(addr: SnesAddress) -> bool {
        matches!(addr.bank & 0x7F, 0x00..=0x3F)
    }

    fn read_digit(&mut self) -> u8 {
        if self.mode != Mode::Read {
            return 0x00;
        }
        match self.index {
            None => {
                self.index = Some(0);
                FRAME
            }
            Some(i) if i < DIGITS => {
                self.index = Some(i + 1);
                self.digits[i]
            }
            Some(_) => {
                self.index = None;
                FRAME
            }
        }
    }

    fn write_digit(&mut self, value: u8) {
        let value = value & 0x0F;
        match value {
            0x0D => {
                self.mode = Mode::Read;
                self.index = None;
                return;
            }
            0x0E => {
                self.mode = Mode::Command;
                return;
            }
            // unknown behavior, ignored
            0x0F => return,
            _ => {}
        }
        match self.mode {
            Mode::Write => {
                if let Some(i) = self.index.filter(|&i| i < DIGITS - 1) {
                    self.digits[i] = value;
                    self.index = Some(i + 1);
                    if i + 1 == DIGITS - 1 {
                        // the chip fills in the day of the week itself
                        self.digits = DateTime::from_digits(&self.digits).to_digits();
                        self.index = Some(DIGITS);
                    }
                }
            }
            Mode::Command => match value {
                0x0 => {
                    self.mode = Mode::Write;
                    self.index = Some(0);
                }
                0x4 => {
                    self.mode = Mode::Ready;
                    self.index = None;
                    self.digits = [0; DIGITS];
                }
                _ => self.mode = Mode::Ready,
            },
            Mode::Ready | Mode::Read => {}
        }
    }

    fn tick_second(&mut self) {
        let mut time = DateTime::from_digits(&self.digits);
        time.tick();
        self.digits = time.to_digits();
    }
}

impl CoprocessorChip for Srtc {
    fn read(&mut self, addr: SnesAddress, _mem: CartridgeMemory) -> Option<(u8, AccessSpeed)> {
        (Self::in_banks(addr) && addr.addr == READ_REG)
            .then(|| (self.read_digit(), AccessSpeed::Fast))
    }

    fn write(
        &mut self,
        addr: SnesAddress,
        value: u8,
        _mem: CartridgeMemory,
    ) -> Option<AccessSpeed> {
        (Self::in_banks(addr) && addr.addr == WRITE_REG).then(|| {
            self.write_digit(value);
            AccessSpeed::Fast
        })
    }

    fn step(&mut self, master_cycles: u32) {
        self.subsecond += u64::from(master_cycles) * MASTER_HZ_DEN;
        while self.subsecond >= MASTER_HZ_NUM {
            self.subsecond -= MASTER_HZ_NUM;
            self.tick_second();
        }
    }

    /// The clock keeps running on its battery: only the transfer state is reset
    fn reset(&mut self) {
        self.mode = Mode::Read;
        self.index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::header::RomHeader;
    use crate::cartridge::header::mapping_mode::MappingMode;
    use crate::cartridge::sram::Sram;
    use crate::cartridge::test_rom::create_valid_lorom;
    use common::snes_address::snes_addr;

    /// S-RTC on a board without S-RAM
    struct Board {
        chip: Srtc,
        sram: Sram,
    }

    impl Board {
        fn new() -> Self {
            let rom = create_valid_lorom(0x10000);
            let header = RomHeader::load_header(&rom, MappingMode::LoRom);
            Self {
                chip: Srtc::new(),
                sram: Sram::new(&header),
            }
        }

        fn mem(&mut self) -> CartridgeMemory<'_> {
            CartridgeMemory {
                rom: &[],
                sram: &mut self.sram,
                map: MappingMode::HiRom,
            }
        }

        fn write(&mut self, value: u8) {
            let mem = CartridgeMemory {
                rom: &[],
                sram: &mut self.sram,
                map: MappingMode::HiRom,
            };
            assert!(self.chip.write(snes_addr!(0x00:WRITE_REG), value, mem).is_some());
        }

        fn read(&mut self) -> u8 {
            let mem = CartridgeMemory {
                rom: &[],
                sram: &mut self.sram,
                map: MappingMode::HiRom,
            };
            self.chip.read(snes_addr!(0x00:READ_REG), mem).unwrap().0
        }

        /// Sets the clock like the game does, from the 12 digits
        fn set(&mut self, digits: [u8; 12]) {
            self.write(0x0E);
            self.write(0x00);
            for digit in digits {
                self.write(digit);
            }
        }

        /// Reads the 13 digits like the game does, checking the markers
        fn get(&mut self) -> [u8; DIGITS] {
            self.write(0x0D);
            assert_eq!(self.read(), FRAME);
            let digits = std::array::from_fn(|_| self.read());
            assert_eq!(self.read(), FRAME);
            digits
        }
    }

    /// Digits of 1995-12-31 23:59:58 (a Sunday)
    const NEW_YEARS_EVE: [u8; 12] = [8, 5, 9, 5, 3, 2, 1, 3, 12, 5, 9, 9];

    #[test]
    fn test_set_then_read_computes_the_weekday() {
        let mut board = Board::new();
        board.set(NEW_YEARS_EVE);
        let digits = board.get();
        assert_eq!(digits[..12], NEW_YEARS_EVE);
        assert_eq!(digits[12], 0); // Sunday
    }

    #[test]
    fn test_reads_loop_over_the_markers() {
        let mut board = Board::new();
        board.set(NEW_YEARS_EVE);
        board.write(0x0D);
        assert_eq!(board.read(), FRAME);
        for _ in 0..DIGITS {
            board.read();
        }
        assert_eq!(board.read(), FRAME);
        // the next read restarts the transfer
        assert_eq!(board.read(), FRAME);
        assert_eq!(board.read(), 8); // seconds, ones
    }

    #[test]
    fn test_only_low_nibble_is_used() {
        let mut board = Board::new();
        board.write(0xFE); // $E: command
        board.write(0x40); // $0: set the clock
        for digit in NEW_YEARS_EVE {
            board.write(0xA0 | digit);
        }
        assert_eq!(board.get()[..12], NEW_YEARS_EVE);
    }

    #[test]
    fn test_clear_command() {
        let mut board = Board::new();
        board.set(NEW_YEARS_EVE);
        board.write(0x0E);
        board.write(0x04);
        // not in read mode until $0D is written: reads return 0
        assert_eq!(board.read(), 0x00);
        let digits = board.get();
        assert_eq!(digits[..12], [0; 12]);
    }

    #[test]
    fn test_unknown_command_goes_back_to_ready() {
        let mut board = Board::new();
        board.set(NEW_YEARS_EVE);
        board.write(0x0E);
        board.write(0x07);
        // a digit now does not change the clock
        board.write(0x03);
        assert_eq!(board.get()[..12], NEW_YEARS_EVE);
    }

    #[test]
    fn test_clock_ticks_with_the_master_clock() {
        let mut board = Board::new();
        board.set(NEW_YEARS_EVE);
        // just under one second: nothing changes
        board.chip.step(21_477_272);
        assert_eq!(board.get()[0], 8);
        board.chip.step(1);
        assert_eq!(board.get()[0], 9);
        // one more second: rolls over to 1996-01-01 00:00:00, a Monday
        board.chip.step(21_477_273);
        assert_eq!(board.get(), [0, 0, 0, 0, 0, 0, 1, 0, 1, 6, 9, 9, 1]);
    }

    #[test]
    fn test_leap_day() {
        let mut board = Board::new();
        // 2000-02-28 23:59:59 (year digits 0, 0, 10 since 1000)
        board.set([9, 5, 9, 5, 3, 2, 8, 2, 2, 0, 0, 10]);
        board.chip.tick_second();
        // 2000-02-29, a Tuesday
        assert_eq!(board.get(), [0, 0, 0, 0, 0, 0, 9, 2, 2, 0, 0, 10, 2]);
        // 1900 is not a leap year: 02-28 goes to 03-01
        board.set([9, 5, 9, 5, 3, 2, 8, 2, 2, 0, 0, 9]);
        board.chip.tick_second();
        assert_eq!(board.get()[6..9], [1, 0, 3]);
    }

    #[test]
    fn test_reset_keeps_the_time() {
        let mut board = Board::new();
        board.set(NEW_YEARS_EVE);
        board.chip.reset();
        // after a reset the chip is already in read mode
        assert_eq!(board.read(), FRAME);
        assert_eq!(board.read(), 8);
        assert_eq!(board.get()[..12], NEW_YEARS_EVE);
    }

    #[test]
    fn test_registers_and_banks() {
        let mut board = Board::new();
        let mut chip = Srtc::new();
        assert!(chip.read(snes_addr!(0xBF:0x2800), board.mem()).is_some());
        assert!(chip.read(snes_addr!(0x00:0x2801), board.mem()).is_none());
        assert!(chip.read(snes_addr!(0x40:0x2800), board.mem()).is_none());
        assert!(chip.write(snes_addr!(0x80:0x2801), 0x0D, board.mem()).is_some());
        assert!(chip.write(snes_addr!(0x00:0x2800), 0x0D, board.mem()).is_none());
        assert!(chip.write(snes_addr!(0xC0:0x2801), 0x0D, board.mem()).is_none());
    }
}
