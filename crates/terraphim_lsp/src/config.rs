use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_true")]
    pub edm_scan_enabled: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            debounce_ms: default_debounce_ms(),
            edm_scan_enabled: true,
        }
    }
}

fn default_debounce_ms() -> u64 {
    250
}

fn default_true() -> bool {
    true
}
