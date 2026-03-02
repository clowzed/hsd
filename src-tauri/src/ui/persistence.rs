use super::state::AppSettings;
use std::path::PathBuf;

fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")))
        .join("honest-sign-scanner")
        .join("settings.json")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                tracing::warn!("Failed to read settings file: {}", e);
                AppSettings::default()
            }
        }
    } else {
        AppSettings::default()
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Ошибка создания директории: {}", e))?;
    }
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Ошибка сериализации: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Ошибка записи настроек: {}", e))
}
