use std::env;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use factstr_node::{EventFilter, EventQuery, FactstrPostgresStore, NewEvent};
use napi::bindgen_prelude::BigInt;
use serde_json::json;

fn bigint_to_u64(value: &BigInt) -> u64 {
    let (sign_bit, unsigned_value, lossless) = value.get_u64();
    assert!(!sign_bit, "expected non-negative BigInt");
    assert!(lossless, "expected lossless BigInt");
    unsigned_value
}

fn option_bigint_to_u64(value: &Option<BigInt>) -> Option<u64> {
    value.as_ref().map(bigint_to_u64)
}

fn postgres_database_url() -> Option<String> {
    env::var("DATABASE_URL").ok()
}

fn run_postgres_node_smoke_suite(database_url: &str) {
    static POSTGRES_NODE_SMOKE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = POSTGRES_NODE_SMOKE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let build_status = Command::new("npm")
        .args(["run", "build"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("npm should be available to build factstr-node");
    assert!(
        build_status.success(),
        "factstr-node build should succeed before postgres smoke"
    );

    let pack_status = Command::new("npm")
        .args(["run", "pack:local"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("npm should be available to pack the main factstr-node package");
    assert!(
        pack_status.success(),
        "factstr-node main package pack should succeed before postgres smoke"
    );

    let prebuilt_status = Command::new("npm")
        .args(["run", "pack:prebuilt:current"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("npm should be available to pack the current prebuilt factstr-node package");
    assert!(
        prebuilt_status.success(),
        "factstr-node current prebuilt package pack should succeed before postgres smoke"
    );

    let install_status = Command::new("npm")
        .args(["--prefix", "smoke", "run", "install:packed"])
        .env("npm_config_cache", "/tmp/factstr-npm-cache")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("npm should be available to install the packed factstr-node smoke package");
    assert!(
        install_status.success(),
        "factstr-node packed smoke install should succeed before postgres smoke"
    );

    let smoke_status = Command::new("npm")
        .args(["--prefix", "smoke", "run", "smoke"])
        .env("FACTSTR_NODE_POSTGRES_DATABASE_URL", database_url)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("npm should be available to run the factstr-node smoke suite");
    assert!(
        smoke_status.success(),
        "factstr-node postgres smoke should succeed"
    );
}

fn unique_postgres_account_id() -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    format!(
        "factstr-node-postgres-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed),
    )
}

#[test]
fn append_query_and_append_if_shapes_match_postgres_bindings() {
    let Some(database_url) = postgres_database_url() else {
        eprintln!("Skipping factstr-node postgres binding test: DATABASE_URL is not set.");
        return;
    };

    let account_id = unique_postgres_account_id();
    let store = FactstrPostgresStore::new(database_url).expect("postgres store should connect");

    let append_result = store
        .append(vec![NewEvent {
            event_type: "account-opened".to_owned(),
            payload: json!({ "accountId": account_id, "owner": "Rico" }),
        }])
        .expect("append should succeed");

    assert!(bigint_to_u64(&append_result.first_sequence_number) >= 1);
    assert_eq!(
        bigint_to_u64(&append_result.first_sequence_number),
        bigint_to_u64(&append_result.last_sequence_number)
    );
    assert_eq!(bigint_to_u64(&append_result.committed_count), 1);

    let context_query = EventQuery {
        filters: Some(vec![EventFilter {
            event_types: Some(vec!["account-opened".to_owned()]),
            payload_predicates: Some(vec![json!({ "accountId": account_id })]),
        }]),
        min_sequence_number: None,
    };

    let query_result = store
        .query(context_query.clone())
        .expect("query should succeed");
    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.event_records[0].event_type, "account-opened");
    assert!(
        option_bigint_to_u64(&query_result.current_context_version).is_some(),
        "expected postgres query to return a current_context_version",
    );

    let append_if_success = store
        .append_if(
            vec![NewEvent {
                event_type: "account-tagged".to_owned(),
                payload: json!({ "accountId": account_id, "tag": "vip" }),
            }],
            context_query.clone(),
            query_result.current_context_version.clone(),
        )
        .expect("append_if success should return a result");
    assert!(append_if_success.append_result.is_some());
    assert!(append_if_success.conflict.is_none());

    let append_if_conflict = store
        .append_if(
            vec![NewEvent {
                event_type: "account-tagged".to_owned(),
                payload: json!({ "accountId": account_id, "tag": "duplicate" }),
            }],
            context_query,
            Some(0u64.into()),
        )
        .expect("append_if conflict should stay explicit");
    assert!(append_if_conflict.append_result.is_none());
    let conflict = append_if_conflict
        .conflict
        .expect("conflict should be present");
    assert_eq!(
        option_bigint_to_u64(&conflict.expected_context_version),
        Some(0),
    );
    assert_eq!(
        option_bigint_to_u64(&conflict.actual_context_version),
        option_bigint_to_u64(&query_result.current_context_version),
    );
}

#[test]
fn postgres_stream_bindings_match_the_smoke_contract() {
    let Some(database_url) = postgres_database_url() else {
        eprintln!("Skipping factstr-node postgres smoke binding test: DATABASE_URL is not set.");
        return;
    };

    run_postgres_node_smoke_suite(&database_url);
}
