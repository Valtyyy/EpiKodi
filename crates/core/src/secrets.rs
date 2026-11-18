//! Write-only secrets (ARCHITECTURE §9): the UI learns whether a secret is
//! configured and where it comes from, never its value.

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SecretKey {
    TmdbApiKey,
}

impl SecretKey {
    pub const ALL: [SecretKey; 1] = [SecretKey::TmdbApiKey];

    /// Stable identifier, also used as the keyring user name.
    pub fn id(self) -> &'static str {
        match self {
            Self::TmdbApiKey => "tmdb_api_key",
        }
    }

    pub fn env_var(self) -> &'static str {
        match self {
            Self::TmdbApiKey => "OPEN_MOSAIK_TMDB_API_KEY",
        }
    }
}

/// A secret in memory. Deliberately no `Serialize`, no `Display`, and a masked
/// `Debug`, so it cannot end up in a log, an error or an IPC payload by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// For the code that actually uses the secret (an HTTP header, say).
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue(***)")
    }
}

/// Why the secret backend cannot be used. Messages never contain a secret.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    #[error("Trousseau verrouillé : déverrouillez le trousseau de la session, puis réessayez")]
    Locked,
    #[error("Trousseau indisponible : {0}")]
    Unavailable(String),
}

/// Secret storage port (OS keyring). Absent entries are not errors.
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Whether a value is stored.
    async fn status(&self, key: SecretKey) -> Result<bool, SecretError>;
    async fn set(&self, key: SecretKey, value: SecretValue) -> Result<(), SecretError>;
    /// Removing an absent entry succeeds.
    async fn clear(&self, key: SecretKey) -> Result<(), SecretError>;
    /// Reserved for backend consumers (e.g. a metadata provider). Never expose
    /// it through a Tauri command or an HTTP route.
    async fn get(&self, key: SecretKey) -> Result<Option<SecretValue>, SecretError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SecretSource {
    Env,
    Keyring,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SecretState {
    Ok,
    EnvLocked,
    KeyringLocked,
    Unavailable,
}

/// What the UI may know about a secret. Never holds the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export, optional_fields))]
pub struct SecretStatus {
    pub key: SecretKey,
    pub configured: bool,
    pub source: SecretSource,
    pub state: SecretState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
