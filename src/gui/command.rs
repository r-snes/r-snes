use egui_sdl2::sdl2::keyboard::{Keycode, Mod};

use super::GuiFrameData;

/// A user action, reachable both from the menu bar and from a keyboard shortcut.
///
/// This is the single source of truth for shortcuts: key presses are matched
/// against [`Command::shortcut`], and the menu bar displays that same binding,
/// so the two can never drift apart. Adding an action means adding a variant
/// here, its shortcut, and its handling in `Gui::execute`.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Command {
    /// Pick a ROM with a file dialog and load it (closes the current game).
    OpenRom,
    /// Close the current game. On the idle screen, Escape quits instead.
    CloseRom,
    /// Quit the emulator.
    Quit,
    /// Reset the running game, like the console's reset button.
    Reset,
    /// Pause or resume emulation.
    TogglePause,
    /// Show or hide the ROM information window.
    ToggleRomInfo,
    /// Pick a Lua plugin with a file dialog. Only possible with no game running,
    /// since the plugin is injected when the next ROM starts.
    #[cfg(feature = "plugins")]
    LoadPlugin,
    /// Run the `default` action of the loaded plugin.
    #[cfg(feature = "plugins")]
    RunPluginDefault,
}

/// A key, optionally combined with Ctrl.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Shortcut {
    ctrl: bool,
    key: Keycode,
}

impl Shortcut {
    const fn plain(key: Keycode) -> Self {
        Self { ctrl: false, key }
    }

    const fn ctrl(key: Keycode) -> Self {
        Self { ctrl: true, key }
    }

    /// Whether this key press triggers the shortcut. Ctrl must match exactly
    /// and Alt must be released, so e.g. Ctrl+R doesn't trigger R.
    pub fn matches(self, key: Keycode, keymod: Mod) -> bool {
        let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
        let alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
        key == self.key && ctrl == self.ctrl && !alt
    }

    /// Human-readable form shown next to menu entries, e.g. `Ctrl+L`.
    pub fn label(self) -> String {
        let key = match self.key {
            Keycode::Escape => "Esc".to_owned(),
            key => key.name(),
        };
        if self.ctrl {
            format!("Ctrl+{key}")
        } else {
            key
        }
    }
}

impl Command {
    /// Every command, in the order shortcuts are matched.
    const ALL: &[Command] = &[
        Command::OpenRom,
        Command::CloseRom,
        Command::Quit,
        Command::Reset,
        Command::TogglePause,
        Command::ToggleRomInfo,
        #[cfg(feature = "plugins")]
        Command::LoadPlugin,
        #[cfg(feature = "plugins")]
        Command::RunPluginDefault,
    ];

    pub fn shortcut(self) -> Shortcut {
        match self {
            Command::OpenRom => Shortcut::ctrl(Keycode::L),
            Command::CloseRom => Shortcut::plain(Keycode::Escape),
            Command::Quit => Shortcut::ctrl(Keycode::Q),
            Command::Reset => Shortcut::ctrl(Keycode::R),
            Command::TogglePause => Shortcut::plain(Keycode::P),
            Command::ToggleRomInfo => Shortcut::plain(Keycode::F1),
            #[cfg(feature = "plugins")]
            Command::LoadPlugin => Shortcut::ctrl(Keycode::P),
            #[cfg(feature = "plugins")]
            Command::RunPluginDefault => Shortcut::plain(Keycode::R),
        }
    }

    /// The command bound to this key press, if any.
    pub fn from_key(key: Keycode, keymod: Mod) -> Option<Command> {
        Self::ALL
            .iter()
            .copied()
            .find(|cmd| cmd.shortcut().matches(key, keymod))
    }

    /// Whether the command makes sense in the current state. The menu greys
    /// out disabled entries; the same check filters keyboard shortcuts.
    pub fn is_enabled(self, data: &GuiFrameData) -> bool {
        let rom_loaded = data.rom_info.is_some();
        match self {
            Command::OpenRom | Command::Quit => true,
            Command::CloseRom | Command::Reset | Command::TogglePause | Command::ToggleRomInfo => {
                rom_loaded
            }
            #[cfg(feature = "plugins")]
            Command::LoadPlugin => !rom_loaded,
            #[cfg(feature = "plugins")]
            Command::RunPluginDefault => data.plugin_loaded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_unique() {
        for (i, a) in Command::ALL.iter().enumerate() {
            for b in &Command::ALL[i + 1..] {
                assert_ne!(
                    a.shortcut(),
                    b.shortcut(),
                    "{a:?} and {b:?} share a shortcut"
                );
            }
        }
    }

    #[test]
    fn every_command_is_reachable_from_its_shortcut() {
        for &cmd in Command::ALL {
            let Shortcut { ctrl, key } = cmd.shortcut();
            let keymod = if ctrl { Mod::LCTRLMOD } else { Mod::NOMOD };
            assert_eq!(Command::from_key(key, keymod), Some(cmd));
        }
    }

    #[test]
    fn modifiers_must_match_exactly() {
        assert_eq!(Command::from_key(Keycode::L, Mod::NOMOD), None);
        assert_eq!(
            Command::from_key(Keycode::L, Mod::RCTRLMOD),
            Some(Command::OpenRom)
        );
        assert_eq!(
            Command::from_key(Keycode::L, Mod::LCTRLMOD | Mod::LALTMOD),
            None
        );
        assert_eq!(
            Command::from_key(Keycode::P, Mod::NOMOD),
            Some(Command::TogglePause)
        );
    }

    #[test]
    fn ctrl_r_maps_to_reset_instead_of_plugin_action() {
        assert_eq!(
            Command::from_key(Keycode::R, Mod::LCTRLMOD),
            Some(Command::Reset)
        );
        assert_eq!(
            Command::from_key(Keycode::R, Mod::RCTRLMOD),
            Some(Command::Reset)
        );
        #[cfg(feature = "plugins")]
        assert_eq!(
            Command::from_key(Keycode::R, Mod::NOMOD),
            Some(Command::RunPluginDefault)
        );
    }

    #[test]
    fn labels_are_readable() {
        assert_eq!(Command::OpenRom.shortcut().label(), "Ctrl+L");
        assert_eq!(Command::CloseRom.shortcut().label(), "Esc");
        assert_eq!(Command::ToggleRomInfo.shortcut().label(), "F1");
    }

    #[test]
    fn rom_only_commands_are_disabled_when_idle() {
        let idle = GuiFrameData::default();
        assert!(!Command::CloseRom.is_enabled(&idle));
        assert!(!Command::Reset.is_enabled(&idle));
        assert!(!Command::TogglePause.is_enabled(&idle));
        assert!(!Command::ToggleRomInfo.is_enabled(&idle));
        assert!(Command::OpenRom.is_enabled(&idle));
    }
}
