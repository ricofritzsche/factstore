mod support;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use factstr::{DurableStream, EventRecord, EventStore, EventStoreError, NewEvent};
use factstr_conformance as store_conformance;
use serde_json::json;

use support::{TemporarySchema, clear_append_batches, durable_stream_cursor, insert_append_batch};

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
    let temporary_schema = TemporarySchema::new();
    let first_store = temporary_schema.create_store();
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

    let first_stream = first_store
        .stream_all_durable(
            &DurableStream::new("account-directory"),
            recording_handle(Arc::clone(&first_delivery_log)),
        )
        .expect("durable stream should succeed");

    let first_batches = wait_for_delivery_count(&first_delivery_log, 1);
    assert_eq!(batch_sequences(&first_batches), vec![vec![1, 2]]);
    assert_eq!(
        durable_stream_cursor(temporary_schema.database_url(), "account-directory").1,
        2
    );

    first_stream.unsubscribe();
    drop(first_store);

    let reopened_store = temporary_schema.create_store();
    let reopened_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _reopened_stream = reopened_store
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
    assert_eq!(
        durable_stream_cursor(temporary_schema.database_url(), "account-directory").1,
        3
    );
}

#[test]
fn replay_setup_failure_does_not_strand_a_durable_stream() {
    let temporary_schema = TemporarySchema::new();
    let store = temporary_schema.create_store();
    let recovery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");
    drop(store);

    clear_append_batches(temporary_schema.database_url());

    let store = temporary_schema.create_store();
    let error = match store.stream_all_durable(
        &DurableStream::new("setup-failure"),
        recording_handle(Arc::clone(&recovery_log)),
    ) {
        Ok(_) => panic!("invalid replay history should fail durable stream"),
        Err(error) => error,
    };
    assert!(matches!(error, EventStoreError::BackendFailure { .. }));

    insert_append_batch(temporary_schema.database_url(), 1, 1);

    let _stream = store
        .stream_all_durable(
            &DurableStream::new("setup-failure"),
            recording_handle(Arc::clone(&recovery_log)),
        )
        .expect("retry should succeed after fixing replay history");

    let delivered_batches = wait_for_delivery_count(&recovery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1]]);
    assert_eq!(
        durable_stream_cursor(temporary_schema.database_url(), "setup-failure").1,
        1
    );
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
        assert!(
            delivered_batches(delivery_log).is_empty(),
            "expected no delivery"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn batch_sequences(delivered_batches: &[Vec<EventRecord>]) -> Vec<Vec<u64>> {
    delivered_batches
        .iter()
        .map(|batch| {
            batch
                .iter()
                .map(|event_record| event_record.sequence_number)
                .collect()
        })
        .collect()
}
