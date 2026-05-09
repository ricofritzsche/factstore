use sqlx::SqlitePool;

pub(crate) const STORE_FORMAT_VERSION: &str = "1";
pub(crate) const APPEND_BATCH_BOUNDARY_FORMAT_KEY: &str = "append_batch_boundary_format";
pub(crate) const APPEND_BATCH_BOUNDARY_FORMAT_SPARSE_V1: &str = "sparse_v1";

pub(crate) async fn initialize_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let is_new_database = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'events'",
    )
    .fetch_one(pool)
    .await?
        == 0;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            sequence_number INTEGER PRIMARY KEY,
            occurred_at TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS append_batches (
            first_sequence_number INTEGER PRIMARY KEY,
            last_sequence_number INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS store_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS subscriber_cursors (
            subscriber_id TEXT PRIMARY KEY,
            event_query TEXT NOT NULL,
            last_processed_sequence_number INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_occurred_at ON events(occurred_at)")
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO store_metadata (key, value)
         VALUES ('store_format_version', ?1)
         ON CONFLICT(key) DO NOTHING",
    )
    .bind(STORE_FORMAT_VERSION)
    .execute(pool)
    .await?;

    if is_new_database {
        sqlx::query(
            "INSERT INTO store_metadata (key, value)
             VALUES (?1, ?2)",
        )
        .bind(APPEND_BATCH_BOUNDARY_FORMAT_KEY)
        .bind(APPEND_BATCH_BOUNDARY_FORMAT_SPARSE_V1)
        .execute(pool)
        .await?;
    }

    Ok(())
}
