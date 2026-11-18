//! In-memory fakes of ports, reusable by other crates' tests.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;

use crate::{
    AppError, Clock, Environment, SettingsRepository, WindowSettings, WindowSettingsStore,
};

/// Clock that only moves when told to.
pub struct ManualClock(AtomicU64);

impl ManualClock {
    pub fn new(start_ms: u64) -> Self {
        Self(AtomicU64::new(start_ms))
    }

    pub fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// In-memory `WindowSettingsStore`.
#[derive(Default)]
pub struct MemoryWindowStore(Mutex<Option<WindowSettings>>);

impl MemoryWindowStore {
    pub fn with(settings: WindowSettings) -> Self {
        Self(Mutex::new(Some(settings)))
    }

    pub fn saved(&self) -> Option<WindowSettings> {
        *self.0.lock().unwrap()
    }
}

impl WindowSettingsStore for MemoryWindowStore {
    fn load(&self) -> Result<Option<WindowSettings>, AppError> {
        Ok(self.saved())
    }

    fn save(&self, settings: &WindowSettings) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(*settings);
        Ok(())
    }
}

/// In-memory `SettingsRepository` holding raw JSON strings.
#[derive(Default)]
pub struct MemorySettingsRepository(Mutex<BTreeMap<String, String>>);

impl MemorySettingsRepository {
    pub fn raw(&self, key: &str) -> Option<String> {
        self.0.lock().unwrap().get(key).cloned()
    }

    pub fn put_raw(&self, key: &str, raw_json: &str) {
        self.0.lock().unwrap().insert(key.into(), raw_json.into());
    }
}

#[async_trait]
impl SettingsRepository for MemorySettingsRepository {
    async fn load_all(&self) -> Result<Vec<(String, String)>, AppError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    async fn save(&self, key: &str, raw_json: &str) -> Result<(), AppError> {
        self.put_raw(key, raw_json);
        Ok(())
    }
}

/// In-memory `Environment`.
#[derive(Default)]
pub struct MemoryEnvironment(Mutex<BTreeMap<String, String>>);

impl MemoryEnvironment {
    pub fn with(vars: &[(&str, &str)]) -> Self {
        Self(Mutex::new(
            vars.iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        ))
    }
}

impl Environment for MemoryEnvironment {
    fn var(&self, name: &str) -> Option<String> {
        self.0.lock().unwrap().get(name).cloned()
    }
}
