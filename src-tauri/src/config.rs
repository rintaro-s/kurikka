use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_server_url")]
    pub multiplayer_server_url: String,
    #[serde(default = "default_widget_offset")]
    pub widget_y_offset: i32,
    #[serde(default = "default_widget_unit_size")]
    pub widget_unit_size: i32,
}

fn default_server_url() -> String {
    "".to_string() // 空の場合はマルチプレイ無効
}

fn default_widget_offset() -> i32 {
    100
}

fn default_widget_unit_size() -> i32 {
    6 // デフォルトのユニットサイズ(中)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            multiplayer_server_url: default_server_url(),
            widget_y_offset: default_widget_offset(),
            widget_unit_size: default_widget_unit_size(),
        }
    }
}

impl AppConfig {
    fn config_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "ClickerClicker", "ClickerClickerClicker")
            .map(|dirs| dirs.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_file_path() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(path) = Self::config_file_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                fs::write(path, json).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
        Err("Failed to save config".to_string())
    }
}
