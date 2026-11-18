use serde::{Deserialize, Serialize};

macro_rules! newtype_custom_default {
    (
        $(
            $(#$attr:tt)*
            $vis:vis struct $newty:ident($wrapped:ty) = $def:expr;
        )+
    ) => {
        $(
            $(#$attr)*
            $vis struct $newty(pub $wrapped);

            impl ::core::default::Default for $newty {
                fn default() -> Self {
                    Self($def)
                }
            }
        )+
    }
}

newtype_custom_default! {
    #[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
    pub struct DefaultTrue(bool) = true;

    #[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
    pub struct DefaultFalse(bool) = false;

    #[derive(Serialize, Deserialize)]
    pub struct PauseWhen(Vec<GuiContext>) = vec![
        GuiContext::Navbar, GuiContext::Menu
    ];
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
enum GuiContext {
    Navbar,
    Menu,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// simpler general options
    pub general: GeneralConfig,

    /// keybinds, controller options etc.
    pub input: InputConfig,

    /// options about how we should handle the config file
    pub config: ConfigConfig,
}

#[derive(Serialize, Deserialize)]
pub struct GeneralConfig {
    pause_when: Option<PauseWhen>,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigConfig {}

#[cfg(test)]
mod test {
    use super::*;
}
