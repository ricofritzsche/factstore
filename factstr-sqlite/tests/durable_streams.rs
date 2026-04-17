mod support;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use factstr::{DurableStream, EventRecord, EventStore, EventStoreError, NewEvent};
use factstr_conformance as store_conformance;
use factstr_sqlite::SqliteStore;
use serde_json::json;

use support::{TemporaryDatabaseFile, connect, subscriber_cursor};

#[test]
fn durable_stream_state_is_reused_for_the_same_durable_stream_id() {
    support::run_store_test(
        store_conformance::durable_stream_state_is_reused_for_the_same_durable_stream_id,
    );
}

#[test]
fn durable_replay_respects_event_type_filters() {
    support::run_store_test(store_conformance::durable_replay_respects_event_type_filters);
}

#[test]
fn durable_replay_respects_payload_predicate_filters() {
    support::run_store_test(store_conformance::durable_replay_respects_payload_predicate_filters);
}

#[test]
fn durable_replay_uses_shared_filter_or_and_semantics() {
    support::run_store_test(store_conformance::durable_replay_uses_shared_filter_or_and_semantics);
}

#[test]
fn durable_replay_to_live_boundary_has_no_duplicates_or_gaps() {
    support::run_store_test(
        store_conformance::durable_replay_to_live_boundary_has_no_duplicates_or_gaps,
    );
}

#[test]
fn durable_stream_replay_failure_does_not_advance_cursor_and_retry_replays_from_same_position() {
    support::run_store_test(
        store_conformance::durable_stream_replay_failure_does_not_advance_cursor_and_retry_replays_from_same_position,
    );
}

#[test]
fn durable_replay_panic_does_not_advance_cursor_and_retry_replays_from_same_position() {
    support::run_store_test(
        store_conformance::durable_replay_panic_does_not_advance_cursor_and_retry_replays_from_same_position,
    );
}

#[test]
fn durable_live_failure_does_not_roll_back_append_success_or_advance_cursor() {
    support::run_store_test(
        store_conformance::durable_live_failure_does_not_roll_back_append_success_or_advance_cursor,
    );
}

#[test]
fn durable_unsubscribe_stops_future_live_delivery_but_retains_durable_cursor_state() {
    support::run_store_test(
        store_conformance::durable_unsubscribe_stops_future_live_delivery_but_retains_durable_cursor_state,
    );
}

#[test]
fn durable_cursor_is_persisted_and_reopen_resumes_after_the_stored_cursor() {
    let database_file = TemporaryDatabaseFile::new("durable-subscriptions-reopen");
    let first_store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let first_delivery_log = Arc::new(Mutex::new(Vec::new()));

    first_store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Primary" }),
            ),
        ])
        .expect("historical append should succeed");

    let first_subscription = first_store
        .stream_all_durable(
            &DurableStream::new("account-directory"),
            recording_handle(Arc::clone(&first_delivery_log)),
        )
        .expect("durable subscribe should succeed");

    let first_batches = wait_for_delivery_count(&first_delivery_log, 1);
    assert_eq!(batch_sequences(&first_batches), vec![vec![1, 2]]);
    assert_eq!(read_cursor(database_file.path(), "account-directory").1, 2);

    first_subscription.unsubscribe();
    drop(first_store);

    let reopened_store =
        SqliteStore::open(database_file.path()).expect("sqlite store should reopen");
    let reopened_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _reopened_subscription = reopened_store
        .stream_all_durable(
            &DurableStream::new("account-directory"),
            recording_handle(Arc::clone(&reopened_delivery_log)),
        )
        .expect("durable resubscribe should succeed");

    assert_no_delivery(&reopened_delivery_log);

    reopened_store
        .append(vec![NewEvent::new(
            "account-credited",
            json!({ "accountId": "a1", "amount": 50 }),
        )])
        .expect("live append should succeed");

    let reopened_batches = wait_for_delivery_count(&reopened_delivery_log, 1);
    assert_eq!(batch_sequences(&reopened_batches), vec![vec![3]]);
    assert_eq!(read_cursor(database_file.path(), "account-directory").1, 3);
}

#[test]
fn replay_setup_failure_does_not_strand_a_durable_subscriber() {
    let database_file = TemporaryDatabaseFile::new("durable-subscriptions-setup-failure");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let recovery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");
    drop(store);

    clear_append_batches(database_file.path());

    let store = SqliteStore::open(database_file.path()).expect("sqlite store should reopen");
    let error = match store.stream_all_durable(
        &DurableStream::new("setup-failure"),
        recording_handle(Arc::clone(&recovery_log)),
    ) {
        Ok(_) => panic!("invalid replay payload should fail durable subscribe"),
        Err(error) => error,
    };
    assert!(matches!(error, EventStoreError::BackendFailure { .. }));

    insert_append_batch(database_file.path(), 1, 1);

    let _subscription = store
        .stream_all_durable(
            &DurableStream::new("setup-failure"),
            recording_handle(Arc::clone(&recovery_log)),
        )
        .expect("retry should succeed after fixing replay data");

    let delivered_batches = wait_for_delivery_count(&recovery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1]]);
    assert_eq!(read_cursor(database_file.path(), "setup-failure").1, 1);
}

#[test]
fn durable_replay_rejects_databases_without_append_batch_history() {
    let database_file = TemporaryDatabaseFile::new("durable-subscriptions-missing-history");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Primary" }),
            ),
        ])
        .expect("historical append should succeed");

    clear_append_batches(database_file.path());

    let error = match store
        .stream_all_durable(&DurableStream::new("missing-history"), Arc::new(|_| Ok(())))
    {
        Ok(_) => panic!("durable replay should reject missing append batch history"),
        Err(error) => error,
    };

    match error {
        EventStoreError::BackendFailure { message } => {
            assert!(
                message.contains("append_batches history"),
                "expected explicit append_batches boundary error, got: {message}"
            );
        }
        other => panic!("expected backend failure, got {other:?}"),
    }
}

fn recording_handle(delivery_log: Arc<Mutex<Vec<Vec<EventRecord>>>>) -> factstr::HandleStream {
    Arc::new(move |event_records| {
        delivery_log
            .lock()
            .expect("delivery log lock should succeed")
            .push(event_records);
        Ok(())
    })
}

fn batch_sequences(delivery_log: &[Vec<EventRecord>]) -> Vec<Vec<u64>> {
    delivery_log
        .iter()
        .map(|batch| {
            batch
                .iter()
                .map(|event_record| event_record.sequence_number)
                .collect()
        })
        .collect()
}

fn delivered_batches(delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>) -> Vec<Vec<EventRecord>> {
    delivery_log
        .lock()
        .expect("delivery log lock should succeed")
        .clone()
}

fn wait_for_delivery_count(
    delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>,
    expected_count: usize,
) -> Vec<Vec<EventRecord>> {
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

fn assert_no_delivery(delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>) {
    let deadline = Instant::now() + Duration::from_millis(100);

    while Instant::now() < deadline {
        let delivered_batches = delivered_batches(delivery_log);
        assert_eq!(
            delivered_batches.len(),
            0,
            "expected no delivered batches, got {}",
            delivered_batches.len()
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn read_cursor(database_path: &std::path::Path, subscriber_id: &str) -> (String, u64) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_path).await;
        subscriber_cursor(&mut connection, subscriber_id)
            .await
            .expect("subscriber cursor should exist")
    })
}

fn insert_append_batch(
    database_path: &std::path::Path,
    first_sequence_number: u64,
    last_sequence_number: u64,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_path).await;
        sqlx::query(
            "INSERT INTO append_batches (first_sequence_number, last_sequence_number)
             VALUES (?1, ?2)",
        )
        .bind(first_sequence_number as i64)
        .bind(last_sequence_number as i64)
        .execute(&mut connection)
        .await
        .expect("append batch insert should succeed");
    });
}

fn clear_append_batches(database_path: &std::path::Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_path).await;
        sqlx::query("DELETE FROM append_batches")
            .execute(&mut connection)
            .await
            .expect("append batch clear should succeed");
    });
}
