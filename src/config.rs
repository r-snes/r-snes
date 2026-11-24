use enumset::{EnumSet, EnumSetType, enum_set};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct RSnesConfig {
    /// simpler general options
    #[serde(default)]
    pub general: GeneralConfig,

    /// keybinds, controller options etc.
    #[serde(default)]
    pub input: InputConfig,

    /// options about how we should handle the config file
    #[serde(default)]
    pub config: ConfigConfig,
}

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct GeneralConfig {
    pause_when: PauseWhen,
}

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct InputConfig {}

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct ConfigConfig {}
#[derive(Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct PauseWhen(pub EnumSet<GuiContext>);

impl Default for PauseWhen {
    fn default() -> Self {
        Self(enum_set![GuiContext::Navbar | GuiContext::Menu])
    }
}

#[derive(Deserialize, Serialize, EnumSetType, Debug)]
#[serde(rename_all = "snake_case")]
#[enumset(serialize_repr = "list")]
pub enum GuiContext {
    Navbar,
    Menu,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    NoConfigPath,
    IoError(std::io::Error),
    TomlError(toml::de::Error),
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoConfigPath => write!(f, "Failed to resolve a system-specific config folder"),
            Self::IoError(_) => write!(f, "Failed to open config file for reading"),
            Self::TomlError(_) => write!(f, "Failed to deserialize config file as an RSnesConfig"),
        }
    }
}
impl std::error::Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoConfigPath => None,
            Self::IoError(e) => Some(e),
            Self::TomlError(e) => Some(e),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "R-SNES", "R-SNES")
        .map(|d| d.config_dir().to_owned().join("r-snes_config.toml"))
}

pub fn read_config(path: Option<impl AsRef<Path>>) -> Result<RSnesConfig, ConfigLoadError> {
    let path = path.ok_or(ConfigLoadError::NoConfigPath)?;
    let contents = std::fs::read(path).map_err(ConfigLoadError::IoError)?;
    let cfg = toml::de::from_slice(&contents).map_err(ConfigLoadError::TomlError)?;

    Ok(cfg)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_config_parses_default_values() {
        assert_eq!(
            toml::de::from_str::<RSnesConfig>(""),
            Ok(Default::default())
        )
    }

    #[test]
    fn parse_pause_when() {
        assert_eq!(
            toml::de::from_str::<RSnesConfig>("general.pause_when = [\"menu\"]"),
            Ok(RSnesConfig {
                general: GeneralConfig {
                    pause_when: PauseWhen(enum_set![GuiContext::Menu]),
                    ..Default::default()
                },
                ..Default::default()
            })
        );
        assert_eq!(
            toml::de::from_str::<RSnesConfig>("general.pause_when = []"),
            Ok(RSnesConfig {
                general: GeneralConfig {
                    pause_when: PauseWhen(enum_set![]),
                    ..Default::default()
                },
                ..Default::default()
            })
        );
        assert_eq!(
            toml::de::from_str::<RSnesConfig>("general.pause_when = [\"navbar\", \"menu\"]"),
            Ok(RSnesConfig {
                general: GeneralConfig {
                    pause_when: PauseWhen(enum_set![GuiContext::Navbar | GuiContext::Menu]),
                    ..Default::default()
                },
                ..Default::default()
            })
        );
    }
}
