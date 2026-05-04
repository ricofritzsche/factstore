use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use factstr_node::{EventFilter, EventQuery, FactstrSqliteStore, NewEvent};
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

struct TemporaryDatabasePath {
    directory_path: PathBuf,
    database_path: PathBuf,
}

impl TemporaryDatabasePath {
    fn new() -> Self {
        static SQLITE_TEST_DIRECTORY_COUNTER: AtomicU64 = AtomicU64::new(0);

        for _ in 0..100 {
            let unique_suffix = format!(
                "{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("current time should be after unix epoch")
                    .as_nanos(),
                SQLITE_TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::Relaxed),
            );
            let directory_path =
                env::temp_dir().join(format!("factstr-node-sqlite-{unique_suffix}"));

            match fs::create_dir(&directory_path) {
                Ok(()) => {
                    let database_path = directory_path.join("factstr.sqlite");
                    assert!(
                        !database_path.exists(),
                        "sqlite test database path should not exist before opening the store",
                    );

                    return Self {
                        directory_path,
                        database_path,
                    };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    continue;
                }
                Err(error) => {
                    panic!("temporary sqlite directory should be created: {error}");
                }
            }
        }

        panic!("failed to create a unique temporary sqlite test directory");
    }

    fn database_path(&self) -> &Path {
        &self.database_path
    }
}

impl Drop for TemporaryDatabasePath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.database_path);
        let _ = fs::remove_dir_all(&self.directory_path);
    }
}

#[test]
fn append_returns_the_append_result_shape() {
    let database_path = TemporaryDatabasePath::new();
    let store =
        FactstrSqliteStore::new(database_path.database_path().to_string_lossy().into_owned())
            .expect("sqlite store should open");

    let append_result = store
        .append(vec![NewEvent {
            event_type: "account-opened".to_owned(),
            payload: json!({ "accountId": "a-1" }),
        }])
        .expect("append should succeed");

    assert_eq!(bigint_to_u64(&append_result.first_sequence_number), 1);
    assert_eq!(bigint_to_u64(&append_result.last_sequence_number), 1);
    assert_eq!(bigint_to_u64(&append_result.committed_count), 1);
}

#[test]
fn query_returns_the_query_result_shape() {
    let database_path = TemporaryDatabasePath::new();
    let store =
        FactstrSqliteStore::new(database_path.database_path().to_string_lossy().into_owned())
            .expect("sqlite store should open");
    store
        .append(vec![NewEvent {
            event_type: "money-deposited".to_owned(),
            payload: json!({ "accountId": "a-1", "amount": 25 }),
        }])
        .expect("append should succeed");

    let query_result = store
        .query(EventQuery {
            filters: Some(vec![EventFilter {
                event_types: Some(vec!["money-deposited".to_owned()]),
                payload_predicates: Some(vec![json!({ "accountId": "a-1" })]),
            }]),
            min_sequence_number: None,
        })
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(
        option_bigint_to_u64(&query_result.last_returned_sequence_number),
        Some(1)
    );
    assert_eq!(
        option_bigint_to_u64(&query_result.current_context_version),
        Some(1)
    );
    assert_eq!(
        bigint_to_u64(&query_result.event_records[0].sequence_number),
        1
    );
    assert!(
        !query_result.event_records[0].occurred_at.is_empty(),
        "expected occurred_at to be present",
    );
    assert_eq!(query_result.event_records[0].event_type, "money-deposited");
}

#[test]
fn append_if_returns_an_explicit_conflict_shape() {
    let database_path = TemporaryDatabasePath::new();
    let store =
        FactstrSqliteStore::new(database_path.database_path().to_string_lossy().into_owned())
            .expect("sqlite store should open");
    store
        .append(vec![NewEvent {
            event_type: "account-opened".to_owned(),
            payload: json!({ "accountId": "a-1" }),
        }])
        .expect("append should succeed");

    let append_if_result = store
        .append_if(
            vec![NewEvent {
                event_type: "money-deposited".to_owned(),
                payload: json!({ "accountId": "a-1", "amount": 25 }),
            }],
            EventQuery {
                filters: Some(vec![EventFilter {
                    event_types: Some(vec!["account-opened".to_owned()]),
                    payload_predicates: None,
                }]),
                min_sequence_number: None,
            },
            Some(0u64.into()),
        )
        .expect("append_if should return an explicit conflict result");

    assert!(append_if_result.append_result.is_none());
    let conflict = append_if_result
        .conflict
        .expect("conflict shape should be present");
    assert_eq!(
        option_bigint_to_u64(&conflict.expected_context_version),
        Some(0)
    );
    assert_eq!(
        option_bigint_to_u64(&conflict.actual_context_version),
        Some(1)
    );
}
