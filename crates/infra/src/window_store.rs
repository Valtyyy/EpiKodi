use std::fs;
use std::path::PathBuf;

use om_core::{AppError, WindowSettings, WindowSettingsStore};

/// Stores [`WindowSettings`] as a small JSON file.
pub struct JsonWindowStore {
    path: PathBuf,
}

impl JsonWindowStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl WindowSettingsStore for JsonWindowStore {
    fn load(&self) -> Result<Option<WindowSettings>, AppError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| AppError::internal(format!("Invalid window settings: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::internal(format!(
                "Cannot read window settings: {e}"
            ))),
        }
    }

    fn save(&self, settings: &WindowSettings) -> Result<(), AppError> {
        let io =
            |e: std::io::Error| AppError::internal(format!("Cannot save window settings: {e}"));
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(io)?;
        }
        let raw = serde_json::to_string_pretty(settings)
            .map_err(|e| AppError::internal(e.to_string()))?;
        fs::write(&self.path, raw).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_missing_file() {
        let dir = std::env::temp_dir().join(format!("om-window-store-{}", std::process::id()));
        let store = JsonWindowStore::new(dir.join("window.json"));
        assert_eq!(store.load().unwrap(), None);

        let s = WindowSettings { decorations: false };
        store.save(&s).unwrap();
        assert_eq!(store.load().unwrap(), Some(s));
        let _ = fs::remove_dir_all(dir);
    }
}
