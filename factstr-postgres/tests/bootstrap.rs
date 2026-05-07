mod support;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use factstr::{DurableStream, EventQuery, EventStore, EventStoreError, NewEvent};
use factstr_postgres::{PostgresBootstrapOptions, PostgresStore};
use serde_json::json;

use support::{TemporaryDatabase, database_url_with_query_parameter};

#[test]
fn bootstrap_creates_a_missing_database_and_returns_a_ready_store() {
    let temporary_database = TemporaryDatabase::new();
    let store = PostgresStore::bootstrap(temporary_database.bootstrap_options())
        .expect("bootstrap should create the missing database");

    let append_result = store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "bootstrap-a1" }),
        )])
        .expect("append should succeed after bootstrap");
    assert_eq!(append_result.first_sequence_number, 1);
    assert_eq!(append_result.last_sequence_number, 1);

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed after bootstrap");
    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));

    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _stream = store
        .stream_all_durable(
            &DurableStream::new("bootstrap-durable"),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("durable stream should succeed after bootstrap");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    assert_eq!(delivered_batches[0].len(), 1);
    assert_eq!(delivered_batches[0][0].sequence_number, 1);

    let reopened_store = PostgresStore::connect(temporary_database.database_url())
        .expect("connect should reuse the bootstrapped database");
    let reopened_query_result = reopened_store
        .query(&EventQuery::all())
        .expect("reopened query should succeed");
    assert_eq!(reopened_query_result.event_records.len(), 1);
}

#[test]
fn bootstrap_is_idempotent_when_the_database_already_exists() {
    let temporary_database = TemporaryDatabase::new();

    let first_store = PostgresStore::bootstrap(temporary_database.bootstrap_options())
        .expect("first bootstrap should create the database");
    first_store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "bootstrap-idempotent" }),
        )])
        .expect("append after first bootstrap should succeed");
    drop(first_store);

    let second_store = PostgresStore::bootstrap(temporary_database.bootstrap_options())
        .expect("second bootstrap should reuse the existing database");
    let second_query_result = second_store
        .query(&EventQuery::all())
        .expect("query after second bootstrap should succeed");
    assert_eq!(second_query_result.event_records.len(), 1);
    assert_eq!(second_query_result.current_context_version, Some(1));

    let append_if_result = second_store
        .append_if(
            vec![NewEvent::new(
                "account-renamed",
                json!({ "accountId": "bootstrap-idempotent", "name": "Primary" }),
            )],
            &EventQuery::all(),
            Some(1),
        )
        .expect("append_if after second bootstrap should succeed");
    assert_eq!(append_if_result.first_sequence_number, 2);
    assert_eq!(append_if_result.last_sequence_number, 2);
}

#[test]
fn bootstrap_rejects_database_name_with_a_slash() {
    assert_invalid_database_name("factstr/demo");
}

#[test]
fn bootstrap_rejects_database_name_with_a_question_mark() {
    assert_invalid_database_name("factstr?demo");
}

#[test]
fn bootstrap_rejects_database_name_with_a_hash() {
    assert_invalid_database_name("factstr#demo");
}

#[test]
fn bootstrap_rejects_database_name_with_whitespace() {
    assert_invalid_database_name("factstr demo");
}

#[test]
fn bootstrap_rejects_database_name_starting_with_a_digit() {
    assert_invalid_database_name("1factstr");
}

#[test]
fn bootstrap_preserves_server_url_query_parameters_when_deriving_the_target_database_url() {
    let temporary_database = TemporaryDatabase::new();
    let server_url_with_query = database_url_with_query_parameter(
        temporary_database.server_url(),
        "application_name",
        "factstr-bootstrap-test",
    );

    let store = PostgresStore::bootstrap(PostgresBootstrapOptions {
        server_url: server_url_with_query,
        database_name: temporary_database.bootstrap_options().database_name,
    })
    .expect("bootstrap should succeed with query parameters on server_url");

    let append_result = store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "bootstrap-query-params" }),
        )])
        .expect("append should succeed after bootstrap with query parameters");
    assert_eq!(append_result.first_sequence_number, 1);

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed after bootstrap with query parameters");
    assert_eq!(query_result.event_records.len(), 1);
}

fn assert_invalid_database_name(database_name: &str) {
    let temporary_database = TemporaryDatabase::new();
    let bootstrap_result = PostgresStore::bootstrap(PostgresBootstrapOptions {
        server_url: temporary_database.server_url().to_owned(),
        database_name: database_name.to_owned(),
    });

    let error = match bootstrap_result {
        Ok(_) => panic!("bootstrap should reject an invalid database_name"),
        Err(error) => error,
    };

    match error {
        EventStoreError::BackendFailure { message } => {
            assert!(
                message.contains("[A-Za-z_][A-Za-z0-9_]*"),
                "unexpected validation message: {message}"
            );
        }
        other => panic!("expected backend failure, got {other:?}"),
    }
}

fn recording_handle(
    delivery_log: Arc<Mutex<Vec<Vec<factstr::EventRecord>>>>,
) -> factstr::HandleStream {
    Arc::new(move |event_records| {
        delivery_log
            .lock()
            .expect("delivery log lock should succeed")
            .push(event_records);
        Ok(())
    })
}

fn delivered_batches(
    delivery_log: &Arc<Mutex<Vec<Vec<factstr::EventRecord>>>>,
) -> Vec<Vec<factstr::EventRecord>> {
    delivery_log
        .lock()
        .expect("delivery log lock should succeed")
        .clone()
}

fn wait_for_delivery_count(
    delivery_log: &Arc<Mutex<Vec<Vec<factstr::EventRecord>>>>,
    expected_count: usize,
) -> Vec<Vec<factstr::EventRecord>> {
    let deadline = Instant::now() + Duration::from_secs(1);

    loop {
        let delivered_batches = delivered_batches(delivery_log);
        if delivered_batches.len() == expected_count {
            return delivered_batches;
        }

        assert!(
            Instant::now() < deadline,
            "expected {expected_count} delivered batches, got {}",
            delivered_batches.len()
        );

        thread::sleep(Duration::from_millis(10));
    }
}
