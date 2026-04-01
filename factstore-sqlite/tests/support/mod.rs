#![allow(dead_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use factstore_sqlite::SqliteStore;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, Row, SqliteConnection};

static NEXT_DATABASE_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn run_store_test<TestFn>(test: TestFn)
where
    TestFn: FnOnce(Box<dyn Fn() -> SqliteStore>),
{
    let database_file = TemporaryDatabaseFile::new("conformance");

    test(Box::new(move || {
        SqliteStore::open(database_file.path()).expect("sqlite store should open")
    }));
}

pub(crate) struct TemporaryDatabaseFile {
    path: PathBuf,
}

impl TemporaryDatabaseFile {
    pub(crate) fn new(test_name: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let next_id = NEXT_DATABASE_ID.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "factstore_sqlite_{test_name}_{timestamp}_{next_id}.db"
        ));

        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryDatabaseFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let wal_file = self.path.with_extension("db-wal");
        let shm_file = self.path.with_extension("db-shm");
        let _ = fs::remove_file(wal_file);
        let _ = fs::remove_file(shm_file);
    }
}

pub(crate) async fn connect(database_path: &Path) -> SqliteConnection {
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(false),
    )
    .await
    .expect("sqlite test connection should succeed")
}

pub(crate) async fn table_exists(connection: &mut SqliteConnection, table_name: &str) -> bool {
    sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1")
        .bind(table_name)
        .fetch_optional(connection)
        .await
        .expect("sqlite_master table lookup should succeed")
        .is_some()
}

pub(crate) async fn index_exists(connection: &mut SqliteConnection, index_name: &str) -> bool {
    sqlx::query("SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?1")
        .bind(index_name)
        .fetch_optional(connection)
        .await
        .expect("sqlite_master index lookup should succeed")
        .is_some()
}

pub(crate) async fn metadata_value(connection: &mut SqliteConnection, key: &str) -> Option<String> {
    sqlx::query("SELECT value FROM store_metadata WHERE key = ?1")
        .bind(key)
        .fetch_optional(connection)
        .await
        .expect("store metadata lookup should succeed")
        .map(|row| row.get::<String, _>("value"))
}

pub(crate) async fn subscriber_cursor(
    connection: &mut SqliteConnection,
    subscriber_id: &str,
) -> Option<(String, u64)> {
    sqlx::query(
        "SELECT event_query, last_processed_sequence_number
         FROM subscriber_cursors
         WHERE subscriber_id = ?1",
    )
    .bind(subscriber_id)
    .fetch_optional(connection)
    .await
    .expect("subscriber cursor lookup should succeed")
    .map(|row| {
        (
            row.get::<String, _>("event_query"),
            row.get::<i64, _>("last_processed_sequence_number") as u64,
        )
    })
}
