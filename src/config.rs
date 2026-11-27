use enumset::{EnumSet, EnumSetType, enum_set};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    pub pause_when: PauseWhen,
}

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
#[serde(deny_unknown_fields)]
pub struct InputConfig {}

#[derive(Default, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
#[serde(deny_unknown_fields)]
pub struct ConfigConfig {}
#[derive(Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub struct PauseWhen(pub EnumSet<GuiContext>);

impl Default for PauseWhen {
    fn default() -> Self {
        Self(enum_set!(
            GuiContext::Navbar |
            GuiContext::PermRequest |
            GuiContext::ErrorPopup |
            GuiContext::SaveState |
            GuiContext::ConfigInfo |
        ))
    }
}

#[derive(Deserialize, Serialize, EnumSetType, Debug)]
#[serde(rename_all = "snake_case")]
#[enumset(serialize_repr = "list")]
pub enum GuiContext {
    Navbar,
    PermRequest,
    ErrorPopup,
    RomInfo,
    SaveState,
    ConfigInfo,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    NoConfigPath,
    IoError(std::io::Error, PathBuf),
    TomlError(toml::de::Error, PathBuf),
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoConfigPath => write!(f, "Failed to resolve a system-specific config folder"),
            Self::IoError(_, path) => write!(f, "Failed to open config file {path:?} for reading"),
            Self::TomlError(_, path) => write!(
                f,
                "Failed to deserialize config file {path:?} as an RSnesConfig"
            ),
        }
    }
}
impl std::error::Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoConfigPath => None,
            Self::IoError(e, _) => Some(e),
            Self::TomlError(e, _) => Some(e),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    Some(
        directories::ProjectDirs::from("", "R-SNES", "R-SNES")?
            .config_dir()
            .join("r-snes_config.toml"),
    )
}

pub fn read_config(path: Option<impl AsRef<Path>>) -> Result<RSnesConfig, ConfigLoadError> {
    let path = path.ok_or(ConfigLoadError::NoConfigPath)?;
    let contents = std::fs::read(&path)
        .map_err(|e| ConfigLoadError::IoError(e, path.as_ref().to_path_buf()))?;
    let cfg = toml::de::from_slice(&contents)
        .map_err(|e| ConfigLoadError::TomlError(e, path.as_ref().to_path_buf()))?;

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
            toml::de::from_str::<RSnesConfig>("general.pause_when = [\"navbar\"]"),
            Ok(RSnesConfig {
                general: GeneralConfig {
                    pause_when: PauseWhen(enum_set![GuiContext::Navbar]),
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
            toml::de::from_str::<RSnesConfig>("general.pause_when = [\"navbar\", \"error_popup\"]"),
            Ok(RSnesConfig {
                general: GeneralConfig {
                    pause_when: PauseWhen(enum_set![GuiContext::Navbar | GuiContext::ErrorPopup]),
                    ..Default::default()
                },
                ..Default::default()
            })
        );
    }

    #[test]
    fn fail_on_unexpected_field() {
        assert!(toml::de::from_str::<RSnesConfig>("non_existent_option = 3").is_err())
    }
}
