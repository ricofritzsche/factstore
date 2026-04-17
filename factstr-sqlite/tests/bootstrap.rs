mod support;

use factstr::{EventQuery, EventStore};
use factstr_sqlite::SqliteStore;

use support::{TemporaryDatabaseFile, connect, index_exists, metadata_value, table_exists};

#[test]
fn opening_a_new_database_creates_schema_and_metadata() {
    let database_file = TemporaryDatabaseFile::new("bootstrap-schema");

    let store =
        SqliteStore::open(database_file.path()).expect("sqlite store should open a new database");
    assert_eq!(store.database_path(), database_file.path());
    drop(store);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_file.path()).await;

        assert!(table_exists(&mut connection, "events").await);
        assert!(table_exists(&mut connection, "store_metadata").await);
        assert!(index_exists(&mut connection, "idx_events_type").await);
        assert!(index_exists(&mut connection, "idx_events_occurred_at").await);

        let store_format_version = metadata_value(&mut connection, "store_format_version").await;
        assert_eq!(store_format_version.as_deref(), Some("1"));
    });
}

#[test]
fn opening_a_new_database_succeeds_inside_a_running_tokio_runtime() {
    let database_file = TemporaryDatabaseFile::new("bootstrap-tokio");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let store = SqliteStore::open(database_file.path())
            .expect("sqlite store should open inside a running tokio runtime");
        assert_eq!(store.database_path(), database_file.path());
    });

    runtime.block_on(async {
        let mut connection = connect(database_file.path()).await;

        assert!(table_exists(&mut connection, "events").await);
        assert!(table_exists(&mut connection, "store_metadata").await);

        let store_format_version = metadata_value(&mut connection, "store_format_version").await;
        assert_eq!(store_format_version.as_deref(), Some("1"));
    });
}

#[test]
fn reopening_the_same_database_succeeds_and_preserves_metadata() {
    let database_file = TemporaryDatabaseFile::new("bootstrap-reopen");

    SqliteStore::open(database_file.path()).expect("first open should succeed");
    SqliteStore::open(database_file.path()).expect("second open should succeed");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_file.path()).await;
        let store_format_version = metadata_value(&mut connection, "store_format_version").await;
        assert_eq!(store_format_version.as_deref(), Some("1"));
    });
}

#[test]
fn wal_mode_is_active_after_bootstrap() {
    let database_file = TemporaryDatabaseFile::new("bootstrap-wal");

    SqliteStore::open(database_file.path()).expect("sqlite store should open a new database");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_file.path()).await;
        let journal_mode = sqlx::query_scalar::<_, String>("PRAGMA journal_mode")
            .fetch_one(&mut connection)
            .await
            .expect("journal_mode pragma should be readable");

        assert_eq!(journal_mode.to_lowercase(), "wal");
    });
}

#[test]
fn no_runtime_methods_remain_deferred_after_phase_d() {
    let database_file = TemporaryDatabaseFile::new("bootstrap-trait");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    let event_store: &dyn EventStore = &store;

    let _subscription = event_store
        .stream_all(std::sync::Arc::new(|_| Ok(())))
        .expect("subscribe_all should succeed");
    let _filtered_subscription = event_store
        .stream_to(&EventQuery::all(), std::sync::Arc::new(|_| Ok(())))
        .expect("subscribe_to should succeed");
}
