//! Validates, persists and broadcasts settings changes.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use tokio::sync::{Mutex as AsyncMutex, watch};

use super::catalog::Settings;
use super::model::{SettingSource, SettingValue, SettingWarning, SettingsPatch};
use super::view::SettingsView;
use crate::AppError;

/// Persistence port. Values are the JSON encoding of a [`SettingValue`].
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn load_all(&self) -> Result<Vec<(String, String)>, AppError>;
    async fn save(&self, key: &str, raw_json: &str) -> Result<(), AppError>;
}

/// Process environment, behind a port so `core` does no ambient I/O.
pub trait Environment: Send + Sync {
    fn var(&self, name: &str) -> Option<String>;
}

/// `network.timeout_ms` becomes `OPEN_MOSAIK_NETWORK_TIMEOUT_MS`.
pub fn env_var_name(key: &str) -> String {
    format!("OPEN_MOSAIK_{}", key.to_uppercase().replace('.', "_"))
}

/// How each setting was resolved, for the settings screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsMeta {
    pub sources: BTreeMap<String, SettingSource>,
    /// Key to human-readable reason, for settings the environment imposes.
    pub locks: BTreeMap<String, String>,
    pub warnings: Vec<SettingWarning>,
}

impl SettingsMeta {
    pub fn source(&self, key: &str) -> SettingSource {
        self.sources
            .get(key)
            .copied()
            .unwrap_or(SettingSource::Default)
    }

    pub fn locked_reason(&self, key: &str) -> Option<&str> {
        self.locks.get(key).map(String::as_str)
    }
}

#[derive(Clone)]
pub struct SettingsService {
    repo: Arc<dyn SettingsRepository>,
    tx: Arc<watch::Sender<Arc<Settings>>>,
    meta: Arc<Mutex<SettingsMeta>>,
    // Serialises updates so two concurrent patches cannot overwrite each other.
    write_lock: Arc<AsyncMutex<()>>,
}

impl SettingsService {
    /// Resolves env > stored > default per key. Never fails: an unreadable
    /// repository or an invalid value falls back to the next level and is
    /// reported in the warnings, because a bad setting must not stop the app
    /// from starting (and thus from letting the user fix it).
    pub async fn load(repo: Arc<dyn SettingsRepository>, env: Arc<dyn Environment>) -> Self {
        let mut meta = SettingsMeta::default();
        let stored: BTreeMap<String, String> = match repo.load_all().await {
            Ok(rows) => rows.into_iter().collect(),
            Err(_) => {
                meta.warnings.push(SettingWarning {
                    key: "storage".into(),
                    message:
                        "Les réglages enregistrés sont illisibles : valeurs par défaut utilisées."
                            .into(),
                });
                BTreeMap::new()
            }
        };

        let mut settings = Settings::default();
        for d in Settings::descriptors() {
            let var = env_var_name(&d.key);
            let mut source = SettingSource::Default;
            if let Some(raw) = env.var(&var) {
                let parsed = d.kind.parse_plain(&raw).ok_or(());
                if parsed
                    .and_then(|v| settings.set(&d.key, &v).map_err(|_| ()))
                    .is_ok()
                {
                    source = SettingSource::Env;
                    meta.locks.insert(
                        d.key.clone(),
                        format!("Imposé par la variable d'environnement {var}"),
                    );
                } else {
                    meta.warnings.push(SettingWarning {
                        key: d.key.clone(),
                        message: format!("Valeur invalide dans {var} : ignorée."),
                    });
                }
            }
            if source == SettingSource::Default
                && let Some(raw) = stored.get(&d.key)
            {
                let applied = serde_json::from_str::<SettingValue>(raw)
                    .map_err(|_| ())
                    .and_then(|v| settings.set(&d.key, &v).map_err(|_| ()));
                if applied.is_ok() {
                    source = SettingSource::Stored;
                } else {
                    meta.warnings.push(SettingWarning {
                        key: d.key.clone(),
                        message: "Valeur enregistrée invalide : valeur par défaut utilisée.".into(),
                    });
                }
            }
            meta.sources.insert(d.key, source);
        }

        let (tx, _) = watch::channel(Arc::new(settings));
        Self {
            repo,
            tx: Arc::new(tx),
            meta: Arc::new(Mutex::new(meta)),
            write_lock: Arc::new(AsyncMutex::new(())),
        }
    }

    pub fn view(&self) -> SettingsView {
        SettingsView::new(self.tx.subscribe())
    }

    pub fn snapshot(&self) -> Arc<Settings> {
        self.tx.borrow().clone()
    }

    pub fn meta(&self) -> SettingsMeta {
        self.meta
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// All-or-nothing validation, then persistence, then broadcast.
    pub async fn update(&self, patch: SettingsPatch) -> Result<Arc<Settings>, AppError> {
        let _guard = self.write_lock.lock().await;
        {
            let meta = self.meta.lock().unwrap_or_else(PoisonError::into_inner);
            for (key, _) in &patch {
                if meta.locks.contains_key(key) {
                    return Err(
                        AppError::validation(key, "Réglage imposé par l'environnement.")
                            .with_suggestion(format!(
                                "Retirez la variable {} pour pouvoir le modifier.",
                                env_var_name(key)
                            )),
                    );
                }
            }
        }
        let mut next = (**self.tx.borrow()).clone();
        for (key, value) in &patch {
            next.set(key, value)?;
        }
        for (key, _) in &patch {
            let Some(value) = next.get(key) else { continue };
            let raw = serde_json::to_string(&value)
                .map_err(|e| AppError::internal(format!("sérialisation du réglage : {e}")))?;
            self.repo.save(key, &raw).await?;
        }
        {
            let mut meta = self.meta.lock().unwrap_or_else(PoisonError::into_inner);
            for (key, _) in &patch {
                meta.sources.insert(key.clone(), SettingSource::Stored);
                meta.warnings.retain(|w| &w.key != key);
            }
        }
        let next = Arc::new(next);
        self.tx.send_replace(next.clone());
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{MemoryEnvironment, MemorySettingsRepository};

    async fn service() -> (SettingsService, Arc<MemorySettingsRepository>) {
        let repo = Arc::new(MemorySettingsRepository::default());
        (
            SettingsService::load(repo.clone(), Arc::new(MemoryEnvironment::default())).await,
            repo,
        )
    }

    #[tokio::test]
    async fn defaults_without_storage() {
        let (svc, _) = service().await;
        let s = svc.snapshot();
        assert!(s.window_decorations);
        assert_eq!(s.network_timeout_ms, 10_000);
        assert!(!s.first_run_completed);
    }

    #[tokio::test]
    async fn update_reaches_view_subscribers_and_storage() {
        let (svc, repo) = service().await;
        let mut view = svc.view();
        svc.update(vec![(
            "interface.window_decorations".into(),
            SettingValue::Bool(false),
        )])
        .await
        .unwrap();
        assert!(!view.current().window_decorations);
        assert!(!view.changed().await.window_decorations);
        assert_eq!(
            repo.raw("interface.window_decorations").unwrap(),
            r#"{"type":"bool","value":false}"#
        );
    }

    #[tokio::test]
    async fn stored_values_override_defaults_and_bad_ones_are_ignored() {
        let repo = Arc::new(MemorySettingsRepository::default());
        repo.put_raw("library.scan_parallelism", r#"{"type":"int","value":4}"#);
        repo.put_raw("network.timeout_ms", r#"{"type":"int","value":5}"#);
        repo.put_raw("nope", "garbage");
        let svc = SettingsService::load(repo, Arc::new(MemoryEnvironment::default())).await;
        assert_eq!(svc.snapshot().scan_parallelism, 4);
        assert_eq!(svc.snapshot().network_timeout_ms, 10_000);
    }

    #[tokio::test]
    async fn out_of_bounds_is_rejected_with_field() {
        let (svc, repo) = service().await;
        let err = svc
            .update(vec![("network.timeout_ms".into(), SettingValue::Int(10))])
            .await
            .unwrap_err();
        assert_eq!(err.code, "validation");
        assert_eq!(err.field.as_deref(), Some("network.timeout_ms"));
        assert!(err.suggestion.unwrap().contains("1000"));
        assert!(repo.raw("network.timeout_ms").is_none());
    }

    #[tokio::test]
    async fn off_step_and_wrong_type_are_rejected() {
        let (svc, _) = service().await;
        assert!(
            svc.update(vec![(
                "network.timeout_ms".into(),
                SettingValue::Int(1_100)
            )])
            .await
            .is_err()
        );
        let err = svc
            .update(vec![(
                "playback.hardware_decoding".into(),
                SettingValue::Int(1),
            )])
            .await
            .unwrap_err();
        assert_eq!(err.field.as_deref(), Some("playback.hardware_decoding"));
    }

    #[tokio::test]
    async fn unknown_key_is_rejected() {
        let (svc, _) = service().await;
        let err = svc
            .update(vec![("nope.nothing".into(), SettingValue::Bool(true))])
            .await
            .unwrap_err();
        assert_eq!(err.field.as_deref(), Some("nope.nothing"));
    }

    #[tokio::test]
    async fn partially_invalid_patch_persists_nothing() {
        let (svc, repo) = service().await;
        let before = svc.snapshot();
        let result = svc
            .update(vec![
                (
                    "interface.window_decorations".into(),
                    SettingValue::Bool(false),
                ),
                ("library.scan_parallelism".into(), SettingValue::Int(99)),
            ])
            .await;
        assert!(result.is_err());
        assert_eq!(svc.snapshot(), before);
        assert!(repo.raw("interface.window_decorations").is_none());
    }

    #[test]
    fn descriptors_cover_every_key_with_matching_defaults() {
        let defaults = Settings::default();
        let descriptors = Settings::descriptors();
        assert_eq!(descriptors.len(), 5);
        for d in descriptors {
            assert_eq!(defaults.get(&d.key), Some(d.default));
        }
    }

    #[tokio::test]
    async fn fixed_view_returns_given_settings() {
        let view = SettingsView::fixed(Settings {
            scan_parallelism: 7,
            ..Settings::default()
        });
        assert_eq!(view.current().scan_parallelism, 7);
    }

    async fn with_env(
        repo: &Arc<MemorySettingsRepository>,
        vars: &[(&str, &str)],
    ) -> SettingsService {
        SettingsService::load(repo.clone(), Arc::new(MemoryEnvironment::with(vars))).await
    }

    #[test]
    fn env_var_name_is_derived_from_key() {
        assert_eq!(
            env_var_name("network.timeout_ms"),
            "OPEN_MOSAIK_NETWORK_TIMEOUT_MS"
        );
    }

    #[tokio::test]
    async fn priority_is_env_then_stored_then_default() {
        let repo = Arc::new(MemorySettingsRepository::default());
        repo.put_raw("network.timeout_ms", r#"{"type":"int","value":3000}"#);
        repo.put_raw("library.scan_parallelism", r#"{"type":"int","value":4}"#);
        let svc = with_env(&repo, &[("OPEN_MOSAIK_NETWORK_TIMEOUT_MS", "5000")]).await;
        let (s, m) = (svc.snapshot(), svc.meta());
        assert_eq!(s.network_timeout_ms, 5000);
        assert_eq!(m.source("network.timeout_ms"), SettingSource::Env);
        assert!(
            m.locked_reason("network.timeout_ms")
                .unwrap()
                .contains("OPEN_MOSAIK_NETWORK_TIMEOUT_MS")
        );
        assert_eq!(s.scan_parallelism, 4);
        assert_eq!(m.source("library.scan_parallelism"), SettingSource::Stored);
        assert_eq!(
            m.source("playback.hardware_decoding"),
            SettingSource::Default
        );
        assert!(m.warnings.is_empty());
    }

    #[tokio::test]
    async fn env_parses_plain_bool() {
        let repo = Arc::new(MemorySettingsRepository::default());
        let svc = with_env(&repo, &[("OPEN_MOSAIK_INTERFACE_WINDOW_DECORATIONS", "0")]).await;
        assert!(!svc.snapshot().window_decorations);
    }

    #[tokio::test]
    async fn env_locked_key_rejects_update_and_persists_nothing() {
        let repo = Arc::new(MemorySettingsRepository::default());
        let svc = with_env(&repo, &[("OPEN_MOSAIK_NETWORK_TIMEOUT_MS", "5000")]).await;
        let err = svc
            .update(vec![
                ("library.scan_parallelism".into(), SettingValue::Int(3)),
                ("network.timeout_ms".into(), SettingValue::Int(2000)),
            ])
            .await
            .unwrap_err();
        assert_eq!(err.field.as_deref(), Some("network.timeout_ms"));
        assert!(
            err.suggestion
                .unwrap()
                .contains("OPEN_MOSAIK_NETWORK_TIMEOUT_MS")
        );
        assert!(repo.raw("library.scan_parallelism").is_none());
        assert_eq!(svc.snapshot().network_timeout_ms, 5000);
    }

    #[tokio::test]
    async fn invalid_stored_falls_back_to_default_with_warning() {
        let repo = Arc::new(MemorySettingsRepository::default());
        repo.put_raw("network.timeout_ms", r#"{"type":"int","value":5}"#);
        repo.put_raw("library.scan_parallelism", "garbage");
        let svc = with_env(&repo, &[]).await;
        let m = svc.meta();
        assert_eq!(svc.snapshot().network_timeout_ms, 10_000);
        assert_eq!(m.source("network.timeout_ms"), SettingSource::Default);
        assert_eq!(m.warnings.len(), 2);
    }

    #[tokio::test]
    async fn invalid_env_falls_back_to_stored_with_warning() {
        let repo = Arc::new(MemorySettingsRepository::default());
        repo.put_raw("network.timeout_ms", r#"{"type":"int","value":3000}"#);
        let svc = with_env(&repo, &[("OPEN_MOSAIK_NETWORK_TIMEOUT_MS", "abc")]).await;
        let m = svc.meta();
        assert_eq!(svc.snapshot().network_timeout_ms, 3000);
        assert_eq!(m.source("network.timeout_ms"), SettingSource::Stored);
        assert!(m.locked_reason("network.timeout_ms").is_none());
        assert_eq!(m.warnings.len(), 1);
        assert_eq!(m.warnings[0].key, "network.timeout_ms");
    }

    #[tokio::test]
    async fn invalid_env_without_stored_falls_back_to_default() {
        let repo = Arc::new(MemorySettingsRepository::default());
        let svc = with_env(&repo, &[("OPEN_MOSAIK_NETWORK_TIMEOUT_MS", "5")]).await;
        assert_eq!(svc.snapshot().network_timeout_ms, 10_000);
        assert_eq!(svc.meta().warnings.len(), 1);
    }

    #[tokio::test]
    async fn update_marks_source_stored_and_clears_warning() {
        let repo = Arc::new(MemorySettingsRepository::default());
        repo.put_raw("network.timeout_ms", "garbage");
        let svc = with_env(&repo, &[]).await;
        svc.update(vec![("network.timeout_ms".into(), SettingValue::Int(2000))])
            .await
            .unwrap();
        let m = svc.meta();
        assert_eq!(m.source("network.timeout_ms"), SettingSource::Stored);
        assert!(m.warnings.is_empty());
    }
}
