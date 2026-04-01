use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use factstore::{DurableStream, EventRecord, EventStore, NewEvent};
use factstore_conformance as store_conformance;
use factstore_memory::MemoryStore;
use serde_json::json;

#[test]
fn durable_stream_state_is_reused_for_the_same_durable_stream_id() {
    store_conformance::durable_stream_state_is_reused_for_the_same_durable_stream_id(
        MemoryStore::new,
    );
}

#[test]
fn durable_replay_respects_event_type_filters() {
    store_conformance::durable_replay_respects_event_type_filters(MemoryStore::new);
}

#[test]
fn durable_replay_respects_payload_predicate_filters() {
    store_conformance::durable_replay_respects_payload_predicate_filters(MemoryStore::new);
}

#[test]
fn durable_replay_uses_shared_filter_or_and_semantics() {
    store_conformance::durable_replay_uses_shared_filter_or_and_semantics(MemoryStore::new);
}

#[test]
fn durable_replay_to_live_boundary_has_no_duplicates_or_gaps() {
    store_conformance::durable_replay_to_live_boundary_has_no_duplicates_or_gaps(MemoryStore::new);
}

#[test]
fn durable_stream_replay_failure_does_not_advance_cursor_and_retry_replays_from_same_position() {
    store_conformance::durable_stream_replay_failure_does_not_advance_cursor_and_retry_replays_from_same_position(MemoryStore::new);
}

#[test]
fn durable_replay_panic_does_not_advance_cursor_and_retry_replays_from_same_position() {
    store_conformance::durable_replay_panic_does_not_advance_cursor_and_retry_replays_from_same_position(MemoryStore::new);
}

#[test]
fn durable_live_failure_does_not_roll_back_append_success_or_advance_cursor() {
    store_conformance::durable_live_failure_does_not_roll_back_append_success_or_advance_cursor(
        MemoryStore::new,
    );
}

#[test]
fn durable_unsubscribe_stops_future_live_delivery_but_retains_durable_cursor_state() {
    store_conformance::durable_unsubscribe_stops_future_live_delivery_but_retains_durable_cursor_state(MemoryStore::new);
}

#[test]
fn durable_stream_state_does_not_survive_store_recreation() {
    let first_store = MemoryStore::new();
    let first_delivery_log = Arc::new(Mutex::new(Vec::new()));

    first_store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");

    let _first_stream = first_store
        .stream_all_durable(
            &DurableStream::new("in-memory-only"),
            recording_handle(Arc::clone(&first_delivery_log)),
        )
        .expect("durable stream should succeed");

    let first_batches = wait_for_delivery_count(&first_delivery_log, 1);
    assert_eq!(batch_sequences(&first_batches), vec![vec![1]]);

    drop(first_store);

    let second_store = MemoryStore::new();
    let second_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _second_stream = second_store
        .stream_all_durable(
            &DurableStream::new("in-memory-only"),
            recording_handle(Arc::clone(&second_delivery_log)),
        )
        .expect("fresh store should start with a fresh in-memory cursor");

    assert_no_delivery(&second_delivery_log);
}

fn recording_handle(delivery_log: Arc<Mutex<Vec<Vec<EventRecord>>>>) -> factstore::HandleStream {
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
