use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use factstore_postgres::PostgresStore;
use sqlx::{Connection, Executor, PgConnection, Row};
use url::Url;

static NEXT_SCHEMA_ID: AtomicU64 = AtomicU64::new(1);

#[allow(dead_code)]
pub(crate) fn run_store_test<TestFn>(test: TestFn)
where
    TestFn: FnOnce(Box<dyn Fn() -> PostgresStore>),
{
    let temporary_schema = TemporarySchema::new();
    let store_database_url = temporary_schema.database_url().to_owned();
    test(Box::new(move || {
        PostgresStore::connect(&store_database_url).expect("postgres store should connect")
    }));
}

#[allow(dead_code)]
pub(crate) fn create_store() -> PostgresStore {
    TemporarySchema::new().create_store()
}

pub(crate) struct TemporarySchema {
    store_database_url: String,
}

impl TemporarySchema {
    pub(crate) fn new() -> Self {
        Self {
            store_database_url: create_store_database_url(),
        }
    }

    pub(crate) fn database_url(&self) -> &str {
        &self.store_database_url
    }

    pub(crate) fn create_store(&self) -> PostgresStore {
        PostgresStore::connect(&self.store_database_url).expect("postgres store should connect")
    }
}

#[allow(dead_code)]
pub(crate) fn durable_stream_cursor(
    store_database_url: &str,
    durable_stream_id: &str,
) -> (String, u64) {
    admin_runtime().block_on(async {
        let mut connection = PgConnection::connect(store_database_url)
            .await
            .expect("postgres test connection should succeed");

        let row = sqlx::query(
            "SELECT event_query, last_processed_sequence_number
             FROM durable_stream_cursors
             WHERE durable_stream_id = $1",
        )
        .bind(durable_stream_id)
        .fetch_one(&mut connection)
        .await
        .expect("durable stream cursor should exist");

        (
            row.get::<String, _>("event_query"),
            row.get::<i64, _>("last_processed_sequence_number") as u64,
        )
    })
}

#[allow(dead_code)]
pub(crate) fn clear_append_batches(store_database_url: &str) {
    admin_runtime().block_on(async {
        let mut connection = PgConnection::connect(store_database_url)
            .await
            .expect("postgres test connection should succeed");

        connection
            .execute("DELETE FROM append_batches")
            .await
            .expect("append_batches should be cleared");
    });
}

#[allow(dead_code)]
pub(crate) fn insert_append_batch(
    store_database_url: &str,
    first_sequence_number: u64,
    last_sequence_number: u64,
) {
    admin_runtime().block_on(async {
        let mut connection = PgConnection::connect(store_database_url)
            .await
            .expect("postgres test connection should succeed");

        sqlx::query(
            "INSERT INTO append_batches (first_sequence_number, last_sequence_number)
             VALUES ($1, $2)",
        )
        .bind(first_sequence_number as i64)
        .bind(last_sequence_number as i64)
        .execute(&mut connection)
        .await
        .expect("append batch should be inserted");
    });
}

fn database_url() -> String {
    env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run factstore-postgres integration tests")
}

fn unique_schema_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let next_id = NEXT_SCHEMA_ID.fetch_add(1, Ordering::Relaxed);

    format!("factstore_test_{timestamp}_{next_id}")
}

fn create_store_database_url() -> String {
    let base_database_url = database_url();
    let schema_name = unique_schema_name();
    admin_runtime().block_on(async {
        let mut connection = PgConnection::connect(&base_database_url)
            .await
            .expect("postgres test connection should succeed");

        connection
            .execute(format!("CREATE SCHEMA \"{schema_name}\"").as_str())
            .await
            .expect("test schema should be created");
    });

    schema_database_url(&base_database_url, &schema_name)
}

fn admin_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build")
}

fn schema_database_url(base_database_url: &str, schema_name: &str) -> String {
    let mut url = Url::parse(base_database_url).expect("DATABASE_URL should be a valid URL");
    url.query_pairs_mut()
        .append_pair("options", &format!("--search_path={schema_name}"));
    url.into()
}
