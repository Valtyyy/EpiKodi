//! SQLite pool and schema migrations.

use std::path::Path;
use std::str::FromStr;

use om_core::AppError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

pub(crate) fn db_error(e: impl std::fmt::Display) -> AppError {
    AppError::new("storage", format!("Base de données indisponible : {e}"))
}

/// Opens (creating if needed) the application database in WAL mode and
/// applies pending migrations.
pub async fn open(path: &Path) -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    connect(options, 5).await
}

/// Throw-away database for tests. A single connection: every connection to
/// `:memory:` would otherwise get its own empty database.
pub async fn open_in_memory() -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .map_err(db_error)?
        .foreign_keys(true);
    connect(options, 1).await
}

async fn connect(
    options: SqliteConnectOptions,
    max_connections: u32,
) -> Result<SqlitePool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await
        .map_err(db_error)?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(db_error)?;
    Ok(pool)
}
