//! Composition root: the only file that knows both services and implementations.

use std::path::Path;
use std::sync::Arc;

use infra::{KeyringSecretStore, ProcessEnvironment, SqliteSettingsRepository, SystemClock};
use om_core::testing::MemorySettingsRepository;
use om_core::{
    AppError, Environment, EventBus, InputService, PingService, SecretStore, SettingWarning,
    SettingsRepository, SettingsService, UiBridge,
};
use tauri::Manager;

/// Every service of the application. Add new services here.
pub struct Services {
    pub events: EventBus,
    pub input: InputService,
    pub ping: PingService,
    pub settings: SettingsService,
}

/// Builds the service graph with the real port implementations.
///
/// Always succeeds (§9: the app must start): if the settings database cannot
/// be opened, settings live in a throw-away in-memory database and the user is
/// warned that changes will not be kept.
pub async fn build(data_dir: &Path, ui: Arc<dyn UiBridge>) -> Services {
    let clock = Arc::new(SystemClock);
    let (repo, warnings) = match open_database(data_dir, clock.clone()).await {
        Ok(repo) => (repo, Vec::new()),
        Err(e) => {
            let warning = SettingWarning {
                key: "storage".into(),
                message: format!(
                    "Base de réglages inaccessible : {}. Les modifications ne seront pas conservées.",
                    e.message
                ),
            };
            let repo: Arc<dyn SettingsRepository> = match infra::db::open_in_memory().await {
                Ok(pool) => Arc::new(SqliteSettingsRepository::new(pool, clock)),
                // Cannot happen short of a broken sqlite build; the plain
                // in-memory fake keeps the app alive anyway.
                Err(_) => Arc::new(MemorySettingsRepository::default()),
            };
            (repo, vec![warning])
        }
    };
    assemble(
        repo,
        Arc::new(ProcessEnvironment),
        Arc::new(KeyringSecretStore::new()),
        ui,
        warnings,
    )
    .await
}

async fn open_database(
    data_dir: &Path,
    clock: Arc<SystemClock>,
) -> Result<Arc<dyn SettingsRepository>, AppError> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| AppError::new("storage", format!("dossier de données inaccessible ({e})")))?;
    let pool = infra::db::open(&data_dir.join("open-mosaik.db")).await?;
    Ok(Arc::new(SqliteSettingsRepository::new(pool, clock)))
}

/// Wires services from already-built ports; tests call it with fakes.
async fn assemble(
    repo: Arc<dyn SettingsRepository>,
    env: Arc<dyn Environment>,
    secrets: Arc<dyn SecretStore>,
    ui: Arc<dyn UiBridge>,
    warnings: Vec<SettingWarning>,
) -> Services {
    let events = EventBus::new();
    Services {
        input: InputService::new(ui),
        ping: PingService::new(Arc::new(SystemClock), events.clone()),
        settings: SettingsService::load_with_warnings(repo, env, secrets, warnings).await,
        events,
    }
}

/// Puts every service in Tauri state. The destructuring has no `..` on purpose:
/// a new field in `Services` that is not managed here does not compile.
pub fn register<R: tauri::Runtime>(app: &impl Manager<R>, services: Services) {
    let Services {
        events,
        input,
        ping,
        settings,
    } = services;

    app.manage(events);
    app.manage(input);
    app.manage(ping);
    app.manage(settings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use om_core::testing::{
        MemoryEnvironment, MemorySecretStore, MemorySettingsRepository, RecordingUiBridge,
    };

    #[tokio::test]
    async fn every_service_is_managed_after_register() {
        let app = tauri::test::mock_app();
        let services = assemble(
            Arc::new(MemorySettingsRepository::default()),
            Arc::new(MemoryEnvironment::default()),
            Arc::new(MemorySecretStore::default()),
            Arc::new(RecordingUiBridge::default()),
            Vec::new(),
        )
        .await;
        register(&app, services);

        // Extend this list for every service added to `Services`.
        assert!(app.try_state::<EventBus>().is_some(), "EventBus");
        assert!(app.try_state::<InputService>().is_some(), "InputService");
        assert!(app.try_state::<PingService>().is_some(), "PingService");
        assert!(
            app.try_state::<SettingsService>().is_some(),
            "SettingsService"
        );
    }
}

#[cfg(test)]
mod fallback_tests {
    use super::*;
    use om_core::testing::RecordingUiBridge;

    #[tokio::test]
    async fn unopenable_database_falls_back_with_a_storage_warning() {
        // A regular file where the data directory should be: creation fails.
        let blocker = std::env::temp_dir().join(format!("om-blocker-{}", std::process::id()));
        std::fs::write(&blocker, b"x").unwrap();
        let services = build(&blocker.join("sub"), Arc::new(RecordingUiBridge::default())).await;
        let snapshot = services.settings.snapshot().await;
        std::fs::remove_file(&blocker).unwrap();
        let w = snapshot
            .warnings
            .iter()
            .find(|w| w.key == "storage")
            .unwrap();
        assert!(
            w.message
                .contains("Les modifications ne seront pas conservées")
        );
    }
}
