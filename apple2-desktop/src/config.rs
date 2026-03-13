use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct EmulatorConfig {
    pub last_disk_path: Option<PathBuf>,
}

impl EmulatorConfig {
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "Apple2Emu", "Apple2Emu")
            .map(|dirs| dirs.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, content);
            }
        }
    }
}