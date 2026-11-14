//! Validates, persists and broadcasts settings changes.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, watch};

use super::catalog::Settings;
use super::model::{SettingValue, SettingsPatch};
use super::view::SettingsView;
use crate::AppError;

/// Persistence port. Values are the JSON encoding of a [`SettingValue`].
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn load_all(&self) -> Result<Vec<(String, String)>, AppError>;
    async fn save(&self, key: &str, raw_json: &str) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct SettingsService {
    repo: Arc<dyn SettingsRepository>,
    tx: Arc<watch::Sender<Arc<Settings>>>,
    // Serialises updates so two concurrent patches cannot overwrite each other.
    write_lock: Arc<Mutex<()>>,
}

impl SettingsService {
    /// Reads the stored values over the defaults. A stored entry that is
    /// unknown or invalid is ignored: the app must always start.
    pub async fn load(repo: Arc<dyn SettingsRepository>) -> Result<Self, AppError> {
        let mut settings = Settings::default();
        for (key, raw) in repo.load_all().await? {
            if let Ok(value) = serde_json::from_str::<SettingValue>(&raw) {
                let _ = settings.set(&key, &value);
            }
        }
        let (tx, _) = watch::channel(Arc::new(settings));
        Ok(Self {
            repo,
            tx: Arc::new(tx),
            write_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn view(&self) -> SettingsView {
        SettingsView::new(self.tx.subscribe())
    }

    pub fn snapshot(&self) -> Arc<Settings> {
        self.tx.borrow().clone()
    }

    /// All-or-nothing validation, then persistence, then broadcast.
    pub async fn update(&self, patch: SettingsPatch) -> Result<Arc<Settings>, AppError> {
        let _guard = self.write_lock.lock().await;
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
        let next = Arc::new(next);
        self.tx.send_replace(next.clone());
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MemorySettingsRepository;

    async fn service() -> (SettingsService, Arc<MemorySettingsRepository>) {
        let repo = Arc::new(MemorySettingsRepository::default());
        (SettingsService::load(repo.clone()).await.unwrap(), repo)
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
        let svc = SettingsService::load(repo).await.unwrap();
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
}
