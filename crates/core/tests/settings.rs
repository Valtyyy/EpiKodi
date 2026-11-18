//! Black-box lock-in of the settings public API: bounds, priority, env locks,
//! invalid stored values, secret handling and hot broadcast.

use std::sync::Arc;
use std::time::Duration;

use om_core::testing::{MemoryEnvironment, MemorySecretStore, MemorySettingsRepository};
use om_core::{
    SecretKey, SecretSource, SecretState, SecretStatus, SecretValue, SettingSource, SettingValue,
    Settings, SettingsService, SettingsView,
};

const TIMEOUT_KEY: &str = "network.timeout_ms";
const TIMEOUT_ENV: &str = "OPEN_MOSAIK_NETWORK_TIMEOUT_MS";
const PARALLELISM_KEY: &str = "library.scan_parallelism";
const DECORATIONS_KEY: &str = "interface.window_decorations";
const HW_KEY: &str = "playback.hardware_decoding";
const SECRET: SecretKey = SecretKey::TmdbApiKey;

struct Fixture {
    repo: Arc<MemorySettingsRepository>,
    store: Arc<MemorySecretStore>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            repo: Arc::new(MemorySettingsRepository::default()),
            store: Arc::new(MemorySecretStore::default()),
        }
    }

    async fn load(&self, vars: &[(&str, &str)]) -> SettingsService {
        SettingsService::load(
            self.repo.clone(),
            Arc::new(MemoryEnvironment::with(vars)),
            self.store.clone(),
        )
        .await
    }
}

async fn service() -> (SettingsService, Fixture) {
    let fx = Fixture::new();
    let svc = fx.load(&[]).await;
    (svc, fx)
}

fn change(key: &str, value: SettingValue) -> Vec<(String, SettingValue)> {
    vec![(key.to_owned(), value)]
}

fn entry<'a>(snapshot: &'a om_core::SettingsSnapshot, key: &str) -> &'a om_core::SettingEntry {
    snapshot
        .entries
        .iter()
        .find(|e| e.descriptor.key == key)
        .unwrap_or_else(|| panic!("no entry for {key}"))
}

// 1. Bounds ---------------------------------------------------------------

#[tokio::test]
async fn min_and_max_bounds_are_accepted() {
    let (svc, _) = service().await;
    svc.update(change(TIMEOUT_KEY, SettingValue::Int(1_000)))
        .await
        .expect("min is accepted");
    assert_eq!(svc.current().network_timeout_ms, 1_000);
    svc.update(change(TIMEOUT_KEY, SettingValue::Int(60_000)))
        .await
        .expect("max is accepted");
    assert_eq!(svc.current().network_timeout_ms, 60_000);
}

#[tokio::test]
async fn below_min_is_rejected_with_field_and_suggestion() {
    let (svc, fx) = service().await;
    let err = svc
        .update(change(TIMEOUT_KEY, SettingValue::Int(999)))
        .await
        .unwrap_err();
    assert_eq!(err.code, "validation");
    assert_eq!(err.field.as_deref(), Some(TIMEOUT_KEY));
    assert!(
        err.suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("1000")
    );
    assert!(fx.repo.raw(TIMEOUT_KEY).is_none());
}

#[tokio::test]
async fn above_max_is_rejected_with_field_and_suggestion() {
    let (svc, fx) = service().await;
    // 60_500 is on the 500 ms step, so only the upper bound is violated.
    let err = svc
        .update(change(TIMEOUT_KEY, SettingValue::Int(60_500)))
        .await
        .unwrap_err();
    assert_eq!(err.field.as_deref(), Some(TIMEOUT_KEY));
    assert!(
        err.suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("60000")
    );
    assert!(fx.repo.raw(TIMEOUT_KEY).is_none());
}

#[tokio::test]
async fn off_step_is_rejected() {
    let (svc, fx) = service().await;
    let err = svc
        .update(change(TIMEOUT_KEY, SettingValue::Int(1_100)))
        .await
        .unwrap_err();
    assert_eq!(err.field.as_deref(), Some(TIMEOUT_KEY));
    assert!(
        err.suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("500")
    );
    assert!(fx.repo.raw(TIMEOUT_KEY).is_none());
}

#[tokio::test]
async fn wrong_type_is_rejected() {
    let (svc, fx) = service().await;
    let err = svc
        .update(change(HW_KEY, SettingValue::Int(1)))
        .await
        .unwrap_err();
    assert_eq!(err.field.as_deref(), Some(HW_KEY));
    assert!(fx.repo.raw(HW_KEY).is_none());
}

#[tokio::test]
async fn unknown_key_is_rejected() {
    let (svc, fx) = service().await;
    let err = svc
        .update(change("does.not.exist", SettingValue::Bool(true)))
        .await
        .unwrap_err();
    assert_eq!(err.field.as_deref(), Some("does.not.exist"));
    assert!(fx.repo.raw("does.not.exist").is_none());
}

// 2. Priority -------------------------------------------------------------

#[tokio::test]
async fn default_only_resolves_to_default_source() {
    let (svc, _) = service().await;
    let meta = svc.meta();
    assert_eq!(svc.current().network_timeout_ms, 10_000);
    assert_eq!(meta.source(TIMEOUT_KEY), SettingSource::Default);
    assert!(meta.locked_reason(TIMEOUT_KEY).is_none());
}

#[tokio::test]
async fn stored_value_resolves_to_stored_source() {
    let fx = Fixture::new();
    fx.repo
        .put_raw(PARALLELISM_KEY, r#"{"type":"int","value":4}"#);
    let svc = fx.load(&[]).await;
    assert_eq!(svc.current().scan_parallelism, 4);
    assert_eq!(svc.meta().source(PARALLELISM_KEY), SettingSource::Stored);
    assert!(svc.meta().locked_reason(PARALLELISM_KEY).is_none());
}

#[tokio::test]
async fn env_wins_over_stored_and_locks_the_key() {
    let fx = Fixture::new();
    fx.repo
        .put_raw(TIMEOUT_KEY, r#"{"type":"int","value":3000}"#);
    let svc = fx.load(&[(TIMEOUT_ENV, "5000")]).await;
    assert_eq!(svc.current().network_timeout_ms, 5_000);
    let meta = svc.meta();
    assert_eq!(meta.source(TIMEOUT_KEY), SettingSource::Env);
    assert!(
        meta.locked_reason(TIMEOUT_KEY)
            .unwrap_or_default()
            .contains(TIMEOUT_ENV)
    );
}

#[tokio::test]
async fn snapshot_reflects_source_and_locked_reason() {
    let fx = Fixture::new();
    fx.repo
        .put_raw(PARALLELISM_KEY, r#"{"type":"int","value":4}"#);
    let svc = fx.load(&[(TIMEOUT_ENV, "5000")]).await;
    let snap = svc.snapshot().await;

    let env_entry = entry(&snap, TIMEOUT_KEY);
    assert_eq!(env_entry.value, SettingValue::Int(5_000));
    assert_eq!(env_entry.source, SettingSource::Env);
    assert!(env_entry.locked_reason.is_some());

    let stored_entry = entry(&snap, PARALLELISM_KEY);
    assert_eq!(stored_entry.value, SettingValue::Int(4));
    assert_eq!(stored_entry.source, SettingSource::Stored);
    assert!(stored_entry.locked_reason.is_none());

    let default_entry = entry(&snap, HW_KEY);
    assert_eq!(default_entry.value, SettingValue::Bool(true));
    assert_eq!(default_entry.source, SettingSource::Default);
    assert!(default_entry.locked_reason.is_none());

    assert!(snap.entries.iter().any(|e| !e.descriptor.visible));
}

// 3. Env lock ------------------------------------------------------------

#[tokio::test]
async fn update_of_env_locked_key_is_rejected_and_persists_nothing() {
    let fx = Fixture::new();
    fx.repo
        .put_raw(TIMEOUT_KEY, r#"{"type":"int","value":3000}"#);
    let svc = fx.load(&[(TIMEOUT_ENV, "5000")]).await;
    let before_stored = fx.repo.raw(TIMEOUT_KEY);
    let before = svc.current();

    let err = svc
        .update(vec![
            (PARALLELISM_KEY.to_owned(), SettingValue::Int(3)),
            (TIMEOUT_KEY.to_owned(), SettingValue::Int(2_000)),
        ])
        .await
        .unwrap_err();

    assert_eq!(err.field.as_deref(), Some(TIMEOUT_KEY));
    assert!(err.suggestion.unwrap_or_default().contains(TIMEOUT_ENV));
    assert_eq!(fx.repo.raw(TIMEOUT_KEY), before_stored);
    assert!(fx.repo.raw(PARALLELISM_KEY).is_none());
    assert_eq!(svc.current(), before);
}

// 4. Invalid stored values after migration ----------------------------------

#[tokio::test]
async fn invalid_stored_json_falls_back_to_default_with_warning() {
    let fx = Fixture::new();
    fx.repo.put_raw(TIMEOUT_KEY, "not json at all");
    let svc = fx.load(&[]).await;
    assert_eq!(svc.current().network_timeout_ms, 10_000);

    let snap = svc.snapshot().await;
    assert_eq!(entry(&snap, TIMEOUT_KEY).source, SettingSource::Default);
    assert!(snap.warnings.iter().any(|w| w.key == TIMEOUT_KEY));
}

#[tokio::test]
async fn out_of_bounds_stored_value_falls_back_to_default_with_warning() {
    let fx = Fixture::new();
    fx.repo
        .put_raw(PARALLELISM_KEY, r#"{"type":"int","value":99}"#);
    let svc = fx.load(&[]).await;
    assert_eq!(svc.current().scan_parallelism, 2);

    let snap = svc.snapshot().await;
    assert_eq!(entry(&snap, PARALLELISM_KEY).value, SettingValue::Int(2));
    assert_eq!(entry(&snap, PARALLELISM_KEY).source, SettingSource::Default);
    assert!(snap.warnings.iter().any(|w| w.key == PARALLELISM_KEY));
}

#[tokio::test]
async fn invalid_stored_values_do_not_stop_loading_other_keys() {
    let fx = Fixture::new();
    fx.repo.put_raw(TIMEOUT_KEY, "garbage");
    fx.repo.put_raw(HW_KEY, r#"{"type":"bool","value":false}"#);
    let svc = fx.load(&[]).await;
    assert!(!svc.current().hardware_decoding);
    assert_eq!(svc.meta().source(HW_KEY), SettingSource::Stored);
    assert_eq!(svc.meta().source(TIMEOUT_KEY), SettingSource::Default);
}

// 5. Secrets ---------------------------------------------------------------

const SECRET_VALUE: &str = "s3cr3t-value-xyz";

#[tokio::test]
async fn secret_never_appears_in_snapshot_status_or_serialisation() {
    let (svc, fx) = service().await;
    let status = svc
        .set_secret(SECRET, SecretValue::new(SECRET_VALUE))
        .await
        .unwrap();
    assert_eq!(fx.store.stored(SECRET).as_deref(), Some(SECRET_VALUE));

    let snap = svc.snapshot().await;
    let snap_json = serde_json::to_string(&snap).unwrap();
    let statuses_json = serde_json::to_string(&svc.secrets_status().await).unwrap();
    let status_json = serde_json::to_string(&status).unwrap();

    for rendered in [
        snap_json,
        statuses_json,
        status_json,
        format!("{snap:?}"),
        format!("{status:?}"),
        format!("{:?}", svc.secrets_status().await),
    ] {
        assert!(!rendered.contains(SECRET_VALUE), "leaked: {rendered}");
    }

    assert!(status.configured);
    assert_eq!(status.source, SecretSource::Keyring);
    assert_eq!(snap.secrets.len(), SecretKey::ALL.len());
}

#[test]
fn secret_value_debug_is_masked() {
    let rendered = format!("{:?}", SecretValue::new(SECRET_VALUE));
    assert_eq!(rendered, "SecretValue(***)");
    assert!(!rendered.contains("s3cr3t"));
}

#[tokio::test]
async fn env_secret_is_locked_and_keyring_is_never_called() {
    let fx = Fixture::new();
    let svc = fx.load(&[("OPEN_MOSAIK_TMDB_API_KEY", SECRET_VALUE)]).await;

    let status: SecretStatus = svc.secret_status(SECRET).await;
    assert!(status.configured);
    assert_eq!(status.source, SecretSource::Env);
    assert_eq!(status.state, SecretState::EnvLocked);

    let snap = svc.snapshot().await;
    let rendered = serde_json::to_string(&snap).unwrap();
    assert!(!rendered.contains(SECRET_VALUE));
    assert_eq!(fx.store.calls(), 0);
}

#[tokio::test]
async fn unavailable_keyring_fails_explicitly_and_stores_nothing() {
    let (svc, fx) = service().await;
    fx.store.set_unavailable(true);

    let err = svc
        .set_secret(SECRET, SecretValue::new(SECRET_VALUE))
        .await
        .unwrap_err();
    assert_eq!(err.code, "secret_keyring_unavailable");
    assert!(!err.message.contains(SECRET_VALUE));
    assert!(!format!("{err:?}").contains(SECRET_VALUE));

    fx.store.set_unavailable(false);
    assert!(fx.store.stored(SECRET).is_none());
    assert!(!svc.secret_status(SECRET).await.configured);
}

// 6. Hot broadcast ----------------------------------------------------------

#[tokio::test]
async fn view_obtained_before_update_sees_the_new_value() {
    let (svc, _) = service().await;
    let view = svc.view();
    let mut waiter_view = svc.view();

    let waiter = tokio::spawn(async move { waiter_view.changed().await });
    svc.update(change(DECORATIONS_KEY, SettingValue::Bool(false)))
        .await
        .unwrap();

    let seen = tokio::time::timeout(Duration::from_secs(5), waiter)
        .await
        .expect("changed() did not resolve")
        .expect("waiter task panicked");
    assert!(!seen.window_decorations);
    assert!(!view.current().window_decorations);
}

#[tokio::test]
async fn view_changed_resolves_after_update_without_hanging() {
    let (svc, _) = service().await;
    let mut view = svc.view();
    svc.update(change(PARALLELISM_KEY, SettingValue::Int(6)))
        .await
        .unwrap();
    let seen = tokio::time::timeout(Duration::from_secs(5), view.changed())
        .await
        .expect("changed() did not resolve");
    assert_eq!(seen.scan_parallelism, 6);
    assert_eq!(view.current().scan_parallelism, 6);
}

#[tokio::test]
async fn fixed_view_returns_given_settings() {
    let view = SettingsView::fixed(Settings {
        scan_parallelism: 7,
        ..Settings::default()
    });
    assert_eq!(view.current().scan_parallelism, 7);
}
