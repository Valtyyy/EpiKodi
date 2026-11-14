//! In-memory fakes of ports, reusable by other crates' tests.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use async_trait::async_trait;

use crate::{
    AppError, Clock, Environment, SecretError, SecretKey, SecretStore, SecretValue,
    SettingsRepository,
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

/// In-memory `SecretStore` that can simulate a locked or missing keyring and
/// counts the calls it receives.
#[derive(Default)]
pub struct MemorySecretStore {
    values: Mutex<BTreeMap<SecretKey, String>>,
    locked: AtomicBool,
    unavailable: AtomicBool,
    calls: AtomicU64,
}

impl MemorySecretStore {
    pub fn set_locked(&self, locked: bool) {
        self.locked.store(locked, Ordering::SeqCst);
    }

    pub fn set_unavailable(&self, unavailable: bool) {
        self.unavailable.store(unavailable, Ordering::SeqCst);
    }

    /// Number of port calls received, to assert the keyring was not touched.
    pub fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn stored(&self, key: SecretKey) -> Option<String> {
        self.values.lock().unwrap().get(&key).cloned()
    }

    fn enter(&self) -> Result<(), SecretError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(SecretError::Unavailable("service absent".into()));
        }
        if self.locked.load(Ordering::SeqCst) {
            return Err(SecretError::Locked);
        }
        Ok(())
    }
}

#[async_trait]
impl SecretStore for MemorySecretStore {
    async fn status(&self, key: SecretKey) -> Result<bool, SecretError> {
        self.enter()?;
        Ok(self.values.lock().unwrap().contains_key(&key))
    }

    async fn set(&self, key: SecretKey, value: SecretValue) -> Result<(), SecretError> {
        self.enter()?;
        self.values
            .lock()
            .unwrap()
            .insert(key, value.expose().to_owned());
        Ok(())
    }

    async fn clear(&self, key: SecretKey) -> Result<(), SecretError> {
        self.enter()?;
        self.values.lock().unwrap().remove(&key);
        Ok(())
    }

    async fn get(&self, key: SecretKey) -> Result<Option<SecretValue>, SecretError> {
        self.enter()?;
        Ok(self
            .values
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .map(SecretValue::new))
    }
}
