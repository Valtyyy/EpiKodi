//! Settings persistence through a real (in-memory) SQLite database.

use std::sync::Arc;

use infra::SqliteSettingsRepository;
use infra::db;
use om_core::testing::{ManualClock, MemoryEnvironment, MemorySecretStore};
use om_core::{SettingSource, SettingValue, SettingsService};

const TIMEOUT_KEY: &str = "network.timeout_ms";
const PARALLELISM_KEY: &str = "library.scan_parallelism";

async fn load(pool: &sqlx::SqlitePool) -> SettingsService {
    SettingsService::load(
        Arc::new(SqliteSettingsRepository::new(
            pool.clone(),
            Arc::new(ManualClock::new(1_000)),
        )),
        Arc::new(MemoryEnvironment::default()),
        Arc::new(MemorySecretStore::default()),
    )
    .await
}

#[tokio::test]
async fn garbage_stored_value_loads_with_default_and_warning() {
    let pool = db::open_in_memory().await.unwrap();
    sqlx::query("INSERT INTO setting (key, value, updated_at) VALUES (?1, 'oops', 0)")
        .bind(TIMEOUT_KEY)
        .execute(&pool)
        .await
        .unwrap();

    let svc = load(&pool).await;
    assert_eq!(svc.current().network_timeout_ms, 10_000);
    assert_eq!(svc.meta().source(TIMEOUT_KEY), SettingSource::Default);

    let snap = svc.snapshot().await;
    assert!(snap.warnings.iter().any(|w| w.key == TIMEOUT_KEY));
}

#[tokio::test]
async fn update_persists_and_survives_a_new_load_on_the_same_pool() {
    let pool = db::open_in_memory().await.unwrap();
    let svc = load(&pool).await;
    svc.update(vec![(PARALLELISM_KEY.to_owned(), SettingValue::Int(5))])
        .await
        .unwrap();

    let stored: String = sqlx::query_scalar("SELECT value FROM setting WHERE key = ?1")
        .bind(PARALLELISM_KEY)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, r#"{"type":"int","value":5}"#);

    let reloaded = load(&pool).await;
    assert_eq!(reloaded.current().scan_parallelism, 5);
    assert_eq!(
        reloaded.meta().source(PARALLELISM_KEY),
        SettingSource::Stored
    );
    assert!(reloaded.meta().warnings.is_empty());
}
