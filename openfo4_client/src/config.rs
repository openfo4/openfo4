use openfo4_config::ConfigSection;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, Default, Deserialize, ConfigSection)]
#[serde(default)]
pub struct Config {
    pub scripting: Scripting,
    pub streaming: Streaming,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, ConfigSection)]
#[serde(default)]
pub struct Scripting {
    /// Enable extra validations in the script compiler and runtime.
    /// Note that this option does not affect the validation pass for papyrus script,
    /// that pass cannot be disabled.
    /// Enabling this option comes at a significant performance penalty, for both
    /// script compilation time and script execution time.
    pub validation: bool,
    /// Enable generation of debug info for compiled scripts.
    /// Enabling this option comes at a significant performance penality for script
    /// compilation.
    pub debug: bool,
}

#[derive(Copy, Clone, Debug, Deserialize, ConfigSection)]
#[serde(default)]
pub struct Streaming {
    ///The radius around the player's current cell which are considered active
    /// for gameplay.
    pub active: u32,
    /// The radius around the player's current cell which are rendered.
    pub visible: u32,
}

impl Default for Streaming {
    fn default() -> Self {
        Self {
            active: 2,
            visible: 3,
        }
    }
}
