use std::fmt;

#[derive(Debug)]
pub enum RomError {
    IoError(std::io::Error),
    FileTooSmall,
}

impl std::error::Error for RomError {}
impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomError::IoError(e) => write!(f, "I/O error: {}", e),
            RomError::FileTooSmall => write!(f, "ROM file too small to be valid."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_display_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let rom_err = RomError::IoError(io_err);

        let msg = format!("{}", rom_err);
        assert!(msg.contains("I/O error:"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_display_file_too_small() {
        let rom_err = RomError::FileTooSmall;

        let msg = format!("{}", rom_err);
        assert_eq!(msg, "ROM file too small to be valid.");
    }

    #[test]
    fn test_debug_format() {
        let rom_err = RomError::FileTooSmall;
        let dbg_msg = format!("{:?}", rom_err);

        assert!(dbg_msg.contains("FileTooSmall"));
    }
}
