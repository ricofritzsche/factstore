mod support;

use factstore::EventStore;
use factstore_conformance as store_conformance;
use factstore_sqlite::SqliteStore;
use serde_json::json;

use support::TemporaryDatabaseFile;

#[test]
fn append_assigns_consecutive_global_sequence_numbers() {
    support::run_store_test(store_conformance::append_assigns_consecutive_global_sequence_numbers);
}

#[test]
fn empty_append_input_returns_typed_error() {
    support::run_store_test(store_conformance::empty_append_input_returns_typed_error);
}

#[test]
fn append_persists_across_reopen() {
    let database_file = TemporaryDatabaseFile::new("append-reopen");

    let first_store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let append_result = first_store
        .append(vec![
            factstore::NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            factstore::NewEvent::new("account-credited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");
    drop(first_store);

    assert_eq!(append_result.first_sequence_number, 1);
    assert_eq!(append_result.last_sequence_number, 2);

    let reopened_store =
        SqliteStore::open(database_file.path()).expect("sqlite store should reopen");
    let query_result = reopened_store
        .query(&factstore::EventQuery::all())
        .expect("query should succeed after reopen");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 2);
}
