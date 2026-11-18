//! SQLite implementation of the settings persistence port.

use std::sync::Arc;

use async_trait::async_trait;
use om_core::{AppError, Clock, SettingsRepository};
use sqlx::{Row, SqlitePool};

use crate::db::db_error;

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool, clock: Arc<dyn Clock>) -> Self {
        Self { pool, clock }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn load_all(&self) -> Result<Vec<(String, String)>, AppError> {
        let rows = sqlx::query("SELECT key, value FROM setting")
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        rows.iter()
            .map(|r| {
                Ok((
                    r.try_get("key").map_err(db_error)?,
                    r.try_get("value").map_err(db_error)?,
                ))
            })
            .collect()
    }

    async fn save(&self, key: &str, raw_json: &str) -> Result<(), AppError> {
        // SQLite integers are signed 64-bit; saturate rather than wrap.
        let now = i64::try_from(self.clock.now_ms()).unwrap_or(i64::MAX);
        sqlx::query(
            "INSERT INTO setting (key, value, updated_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(raw_json)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use om_core::testing::ManualClock;
    use om_core::{SettingSource, SettingValue, SettingsService};

    async fn repo() -> (SqliteSettingsRepository, SqlitePool) {
        let pool = db::open_in_memory().await.unwrap();
        let repo = SqliteSettingsRepository::new(pool.clone(), Arc::new(ManualClock::new(42)));
        (repo, pool)
    }

    #[tokio::test]
    async fn save_then_load_roundtrip() {
        let (repo, _) = repo().await;
        repo.save("a.b", r#"{"type":"bool","value":true}"#)
            .await
            .unwrap();
        assert_eq!(
            repo.load_all().await.unwrap(),
            vec![(
                "a.b".to_owned(),
                r#"{"type":"bool","value":true}"#.to_owned()
            )]
        );
    }

    #[tokio::test]
    async fn save_upserts_and_stamps_time() {
        let (repo, pool) = repo().await;
        repo.save("k", "1").await.unwrap();
        repo.save("k", "2").await.unwrap();
        assert_eq!(
            repo.load_all().await.unwrap(),
            vec![("k".into(), "2".into())]
        );
        let ts: i64 = sqlx::query_scalar("SELECT updated_at FROM setting WHERE key = 'k'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(ts, 42);
    }

    #[tokio::test]
    async fn invalid_raw_value_yields_default_with_warning() {
        let (repo, pool) = repo().await;
        sqlx::query(
            "INSERT INTO setting (key, value, updated_at) VALUES ('network.timeout_ms', 'oops', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let svc = SettingsService::load(
            Arc::new(repo),
            Arc::new(om_core::testing::MemoryEnvironment::default()),
        )
        .await;
        assert_eq!(svc.snapshot().network_timeout_ms, 10_000);
        assert_eq!(
            svc.meta().source("network.timeout_ms"),
            SettingSource::Default
        );
        assert_eq!(svc.meta().warnings.len(), 1);
    }

    #[tokio::test]
    async fn service_update_persists_through_sqlite() {
        let (repo, _) = repo().await;
        let repo = Arc::new(repo);
        let env = Arc::new(om_core::testing::MemoryEnvironment::default());
        let svc = SettingsService::load(repo.clone(), env.clone()).await;
        svc.update(vec![(
            "library.scan_parallelism".into(),
            SettingValue::Int(5),
        )])
        .await
        .unwrap();
        let reloaded = SettingsService::load(repo, env).await;
        assert_eq!(reloaded.snapshot().scan_parallelism, 5);
    }
}
