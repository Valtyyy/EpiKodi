//! Keyring-backed `SecretStore` over the freedesktop Secret Service.
//!
//! Follows docs/spikes/2.6-secret-service.md section 7: lock check before any
//! access, one worker thread per call with a time budget, French messages.

use std::time::Duration;

use async_trait::async_trait;
use om_core::{SecretError, SecretKey, SecretStore, SecretValue};

const SERVICE_NAME: &str = "open-mosaik";
/// `dbus-secret-service` waits up to a year on an unanswered unlock prompt.
const CALL_BUDGET: Duration = Duration::from_secs(8);

pub struct KeyringSecretStore {
    budget: Duration,
}

impl KeyringSecretStore {
    pub fn new() -> Self {
        Self {
            budget: CALL_BUDGET,
        }
    }

    /// Runs `op` on a dedicated thread, bounded by the budget. On timeout the
    /// thread is left behind (the call cannot be cancelled); calls are rare.
    async fn run<T, F>(&self, op: F) -> Result<T, SecretError>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, SecretError> + Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let _ = tx.send(guarded(op));
        });
        match tokio::time::timeout(self.budget, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(SecretError::Unavailable(
                "le fil d'exécution du trousseau s'est arrêté".into(),
            )),
            Err(_) => Err(timeout_error(self.budget)),
        }
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Fails with `Locked` instead of letting the call trigger an unlock prompt.
fn guarded<T>(op: impl FnOnce() -> Result<T, SecretError>) -> Result<T, SecretError> {
    if collection_locked()? {
        return Err(SecretError::Locked);
    }
    op()
}

fn collection_locked() -> Result<bool, SecretError> {
    use dbus_secret_service::{EncryptionType, SecretService};
    // Plain is fine here: only the `Locked` property is read, no secret moves.
    let fail = |e: dbus_secret_service::Error| {
        SecretError::Unavailable(format!("erreur de la plateforme ({e})"))
    };
    let service = SecretService::connect(EncryptionType::Plain).map_err(fail)?;
    let collection = service.get_any_collection().map_err(fail)?;
    collection.is_locked().map_err(fail)
}

fn entry(key: SecretKey) -> Result<keyring::Entry, SecretError> {
    keyring::Entry::new(SERVICE_NAME, key.id()).map_err(|e| map_error(&e))
}

fn timeout_error(budget: Duration) -> SecretError {
    SecretError::Unavailable(format!(
        "délai dépassé ({} s). Le trousseau est peut-être verrouillé et attend une \
         confirmation (invite de déverrouillage non visible).",
        budget.as_secs()
    ))
}

/// Never formats variants that can carry secret bytes (`BadEncoding`).
fn map_error(err: &keyring::Error) -> SecretError {
    use keyring::Error as E;
    let reason = match err {
        E::NoStorageAccess(e) => format!("accès refusé ou impossible ({e})"),
        E::PlatformFailure(e) => format!("erreur de la plateforme ({e})"),
        _ => "erreur inattendue du trousseau".to_owned(),
    };
    SecretError::Unavailable(reason)
}

#[async_trait]
impl SecretStore for KeyringSecretStore {
    async fn status(&self, key: SecretKey) -> Result<bool, SecretError> {
        self.run(move || match entry(key)?.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(map_error(&e)),
        })
        .await
    }

    async fn set(&self, key: SecretKey, value: SecretValue) -> Result<(), SecretError> {
        self.run(move || {
            entry(key)?
                .set_password(value.expose())
                .map_err(|e| map_error(&e))
        })
        .await
    }

    async fn clear(&self, key: SecretKey) -> Result<(), SecretError> {
        self.run(move || match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(map_error(&e)),
        })
        .await
    }

    async fn get(&self, key: SecretKey) -> Result<Option<SecretValue>, SecretError> {
        self.run(move || match entry(key)?.get_password() {
            Ok(v) => Ok(Some(SecretValue::new(v))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(map_error(&e)),
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io(msg: &str) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(std::io::Error::other(msg.to_owned()))
    }

    #[test]
    fn platform_failure_maps_to_unavailable_with_reason() {
        let e = map_error(&keyring::Error::PlatformFailure(io("no bus")));
        assert_eq!(
            e,
            SecretError::Unavailable("erreur de la plateforme (no bus)".into())
        );
        assert_eq!(
            e.to_string(),
            "Trousseau indisponible : erreur de la plateforme (no bus)"
        );
    }

    #[test]
    fn no_storage_access_maps_to_refused() {
        let e = map_error(&keyring::Error::NoStorageAccess(io("dismissed")));
        assert_eq!(
            e,
            SecretError::Unavailable("accès refusé ou impossible (dismissed)".into())
        );
    }

    #[test]
    fn bad_encoding_never_leaks_bytes() {
        let e = map_error(&keyring::Error::BadEncoding(b"topsecret".to_vec()));
        assert!(!e.to_string().contains("topsecret"));
    }

    #[test]
    fn timeout_message_matches_spike() {
        let e = timeout_error(Duration::from_secs(8));
        assert!(
            e.to_string()
                .starts_with("Trousseau indisponible : délai dépassé (8 s).")
        );
    }

    #[test]
    fn locked_message_matches_spike() {
        assert_eq!(
            SecretError::Locked.to_string(),
            "Trousseau verrouillé : déverrouillez le trousseau de la session, puis réessayez"
        );
    }

    #[tokio::test]
    async fn timed_out_call_reports_unavailable() {
        let store = KeyringSecretStore {
            budget: Duration::from_millis(50),
        };
        let r: Result<(), _> = store
            .run(|| {
                std::thread::sleep(Duration::from_millis(500));
                Ok(())
            })
            .await;
        assert!(matches!(r, Err(SecretError::Unavailable(_))));
    }

    // Real session keyring: run manually with `cargo test -p infra -- --ignored`.
    // Overwrites and then deletes the real TMDB entry: do not run with a key you need.
    #[tokio::test]
    #[ignore = "touches the real session keyring"]
    async fn real_keyring_roundtrip() {
        let store = KeyringSecretStore::new();
        let key = SecretKey::TmdbApiKey;
        store
            .set(key, SecretValue::new("test-value"))
            .await
            .unwrap();
        assert!(store.status(key).await.unwrap());
        assert_eq!(
            store.get(key).await.unwrap().unwrap().expose(),
            "test-value"
        );
        store.clear(key).await.unwrap();
        assert!(!store.status(key).await.unwrap());
    }
}
