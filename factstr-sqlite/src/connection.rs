use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};

pub(crate) async fn open_pool(database_path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let connect_options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(connect_options)
        .await
}
