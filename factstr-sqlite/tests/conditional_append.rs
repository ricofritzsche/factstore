mod support;

use factstr::{EventFilter, EventQuery, EventStore, EventStoreError, NewEvent};
use factstr_conformance as store_conformance;
use factstr_sqlite::SqliteStore;
use serde_json::json;

use support::TemporaryDatabaseFile;

#[test]
fn conditional_append_succeeds_for_matching_payload_filtered_context_version() {
    support::run_store_test(
        store_conformance::conditional_append_succeeds_for_matching_payload_filtered_context_version,
    );
}

#[test]
fn conditional_append_fails_for_stale_payload_filtered_context_version() {
    support::run_store_test(
        store_conformance::conditional_append_fails_for_stale_payload_filtered_context_version,
    );
}

#[test]
fn failed_conditional_append_does_not_append_any_part_of_the_batch() {
    support::run_store_test(
        store_conformance::failed_conditional_append_does_not_append_any_part_of_the_batch,
    );
}

#[test]
fn failed_conditional_append_does_not_consume_sequence_numbers_for_later_commits() {
    support::run_store_test(
        store_conformance::failed_conditional_append_does_not_consume_sequence_numbers_for_later_commits,
    );
}

#[test]
fn successful_conditional_append_persists_across_reopen() {
    let database_file = TemporaryDatabaseFile::new("conditional-append-reopen-success");
    let first_store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    first_store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let append_result = first_store
        .append_if(
            vec![NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Rico" }),
            )],
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            Some(1),
        )
        .expect("conditional append should succeed");
    drop(first_store);

    assert_eq!(append_result.first_sequence_number, 2);
    assert_eq!(append_result.last_sequence_number, 2);

    let reopened_store =
        SqliteStore::open(database_file.path()).expect("sqlite store should reopen");
    let query_result = reopened_store
        .query(&EventQuery::all())
        .expect("query should succeed after reopen");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 2);
}

#[test]
fn failed_conditional_append_leaves_database_unchanged_across_reopen() {
    let database_file = TemporaryDatabaseFile::new("conditional-append-reopen-conflict");
    let first_store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    first_store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let conflict = first_store
        .append_if(
            vec![
                NewEvent::new("account-renamed", json!({ "accountId": "a1" })),
                NewEvent::new("account-credited", json!({ "accountId": "a1" })),
            ],
            &EventQuery::all()
                .with_filters([
                    EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
                ])
                .with_min_sequence_number(1),
            None,
        )
        .expect_err("conditional append should fail");
    drop(first_store);

    assert_eq!(
        conflict,
        EventStoreError::ConditionalAppendConflict {
            expected: None,
            actual: Some(1),
        }
    );

    let reopened_store =
        SqliteStore::open(database_file.path()).expect("sqlite store should reopen");
    let query_result = reopened_store
        .query(&EventQuery::all())
        .expect("query should succeed after reopen");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.event_records[0].sequence_number, 1);

    let later_append = reopened_store
        .append(vec![NewEvent::new(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("later append should succeed");

    assert_eq!(later_append.first_sequence_number, 2);
    assert_eq!(later_append.last_sequence_number, 2);
}
