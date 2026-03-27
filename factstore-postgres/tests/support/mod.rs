use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use factstore_postgres::PostgresStore;
use sqlx::{Connection, Executor, PgConnection};

static NEXT_SCHEMA_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn run_store_test<TestFn>(test: TestFn)
where
    TestFn: FnOnce(Box<dyn Fn() -> PostgresStore>),
{
    let Some(base_database_url) = database_url() else {
        eprintln!("skipping postgres test because DATABASE_URL is not set");
        return;
    };

    let schema_name = unique_schema_name();
    let admin_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    admin_runtime.block_on(async {
        let mut connection = PgConnection::connect(&base_database_url)
            .await
            .expect("postgres test connection should succeed");

        connection
            .execute(format!("CREATE SCHEMA \"{schema_name}\"").as_str())
            .await
            .expect("test schema should be created");
    });

    let store_database_url = format!("{base_database_url}?options=--search_path%3D{schema_name}");
    test(Box::new(move || {
        PostgresStore::connect(&store_database_url).expect("postgres store should connect")
    }));
}

fn database_url() -> Option<String> {
    env::var("DATABASE_URL").ok()
}

fn unique_schema_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let next_id = NEXT_SCHEMA_ID.fetch_add(1, Ordering::Relaxed);

    format!("factstore_test_{timestamp}_{next_id}")
}
