#[derive(Default)]
pub struct ConfigUi {
    pub action: Option<ConfigUiAction>
}

pub enum ConfigUiAction {
    Close,
    Reload,
    CreateEmptyConfig,
    CreateDefaultConfig,
    OpenConfigFile,
}
