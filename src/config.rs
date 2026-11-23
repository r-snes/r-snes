use std::path::PathBuf;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Config {
    /// simpler general options
    pub general: GeneralConfig,

    /// keybinds, controller options etc.
    pub input: InputConfig,

    /// options about how we should handle the config file
    pub config: ConfigConfig,
}

#[derive(Default, Deserialize)]
pub struct GeneralConfig {
    pause_when: PauseWhen,
}

#[derive(Default, Deserialize)]
pub struct InputConfig {}

#[derive(Default, Deserialize)]
pub struct ConfigConfig {}
#[derive(Deserialize)]
pub struct PauseWhen(pub Vec<GuiContext>);

impl Default for PauseWhen {
    fn default() -> Self {
        Self(vec![GuiContext::Navbar, GuiContext::Menu])
    }
}

#[derive(Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GuiContext {
    Navbar,
    Menu,
}

pub enum ConfigLoadError {
    NoConfigPath,
    IoError(std::io::Error),
    TomlError(toml::de::Error),
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "R-SNES", "R-SNES")
        .map(|d| d.config_dir().to_owned().join("r-snes_config.toml"))
}

pub fn read_config() -> Result<Config, ConfigLoadError> {
    let path = config_path().ok_or(ConfigLoadError::NoConfigPath)?;
    let contents = std::fs::read(path).map_err(ConfigLoadError::IoError)?;
    let cfg = toml::de::from_slice(&contents).map_err(ConfigLoadError::TomlError)?;

    Ok(cfg)
}

#[cfg(test)]
mod test {
    use super::*;
}
