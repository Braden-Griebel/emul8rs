use serde::{Deserialize, Serialize};

/// Configuration of the emulator
///
/// Includes settings for dealing with some ambigous instructions.
#[derive(Serialize, Deserialize)]
pub struct EmulatorConfig {
    pub instructions_per_second: u64,
    pub shift_use_vy: bool,
    pub jump_offset_use_v0: bool,
    pub store_memory_update_index: bool,
    pub foreground: String,
    pub background: String,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            instructions_per_second: 700,
            shift_use_vy: true,
            jump_offset_use_v0: true,
            store_memory_update_index: false,
            foreground: "000000".to_string(),
            background: "FFFFFF".to_string(),
        }
    }
}
