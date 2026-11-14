//! Composition root: the only file that knows both services and implementations.

use std::path::Path;
use std::sync::Arc;

use infra::{KeyringSecretStore, ProcessEnvironment, SqliteSettingsRepository, SystemClock};
use om_core::{
    AppError, Environment, EventBus, PingService, SecretStore, SettingsRepository, SettingsService,
};
use tauri::Manager;

/// Every service of the application. Add new services here.
pub struct Services {
    pub events: EventBus,
    pub ping: PingService,
    pub settings: SettingsService,
}

/// Builds the service graph with the real port implementations.
pub async fn build(data_dir: &Path) -> Result<Services, AppError> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| AppError::new("storage", format!("Dossier de données inaccessible : {e}")))?;
    let pool = infra::db::open(&data_dir.join("open-mosaik.db")).await?;
    let repo = Arc::new(SqliteSettingsRepository::new(pool, Arc::new(SystemClock)));
    Ok(assemble(
        repo,
        Arc::new(ProcessEnvironment),
        Arc::new(KeyringSecretStore::new()),
    )
    .await)
}

/// Wires services from already-built ports; tests call it with fakes.
async fn assemble(
    repo: Arc<dyn SettingsRepository>,
    env: Arc<dyn Environment>,
    secrets: Arc<dyn SecretStore>,
) -> Services {
    let events = EventBus::new();
    Services {
        ping: PingService::new(Arc::new(SystemClock), events.clone()),
        settings: SettingsService::load(repo, env, secrets).await,
        events,
    }
}

/// Puts every service in Tauri state. The destructuring has no `..` on purpose:
/// a new field in `Services` that is not managed here does not compile.
pub fn register<R: tauri::Runtime>(app: &impl Manager<R>, services: Services) {
    let Services {
        events,
        ping,
        settings,
    } = services;

    app.manage(events);
    app.manage(ping);
    app.manage(settings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use om_core::testing::{MemoryEnvironment, MemorySecretStore, MemorySettingsRepository};

    #[tokio::test]
    async fn every_service_is_managed_after_register() {
        let app = tauri::test::mock_app();
        let services = assemble(
            Arc::new(MemorySettingsRepository::default()),
            Arc::new(MemoryEnvironment::default()),
            Arc::new(MemorySecretStore::default()),
        )
        .await;
        register(&app, services);

        // Extend this list for every service added to `Services`.
        assert!(app.try_state::<EventBus>().is_some(), "EventBus");
        assert!(app.try_state::<PingService>().is_some(), "PingService");
        assert!(
            app.try_state::<SettingsService>().is_some(),
            "SettingsService"
        );
    }
}
