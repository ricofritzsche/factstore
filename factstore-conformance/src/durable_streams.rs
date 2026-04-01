use factstore::{
    DurableStream, EventFilter, EventQuery, EventRecord, EventStore, EventStoreError, NewEvent,
    StreamHandlerError,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::support::{delivered_batches, recording_handle, wait_for_delivery_count};

pub fn durable_stream_state_is_reused_for_the_same_durable_stream_id<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let first_delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Primary" }),
            ),
        ])
        .expect("historical append should succeed");

    let first_stream = store
        .stream_all_durable(
            &DurableStream::new("account-directory"),
            recording_handle(Arc::clone(&first_delivery_log)),
        )
        .expect("durable stream should succeed");

    let first_batches = wait_for_delivery_count(&first_delivery_log, 1);
    assert_eq!(batch_sequences(&first_batches), vec![vec![1, 2]]);

    first_stream.unsubscribe();

    let resumed_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _resumed_stream = store
        .stream_all_durable(
            &DurableStream::new("account-directory"),
            recording_handle(Arc::clone(&resumed_delivery_log)),
        )
        .expect("durable resubscribe should succeed");

    assert_no_delivery(&resumed_delivery_log);

    store
        .append(vec![NewEvent::new(
            "account-credited",
            json!({ "accountId": "a1", "amount": 50 }),
        )])
        .expect("live append should succeed");

    let resumed_batches = wait_for_delivery_count(&resumed_delivery_log, 1);
    assert_eq!(batch_sequences(&resumed_batches), vec![vec![3]]);
}

pub fn durable_replay_respects_event_type_filters<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new(
                "account-credited",
                json!({ "accountId": "a1", "amount": 10 }),
            ),
        ])
        .expect("historical append should succeed");
    store
        .append(vec![NewEvent::new(
            "account-renamed",
            json!({ "accountId": "a1", "name": "Primary" }),
        )])
        .expect("second historical append should succeed");

    let _stream = store
        .stream_to_durable(
            &DurableStream::new("account-types"),
            &EventQuery::all().with_filters([EventFilter::for_event_types([
                "account-opened",
                "account-renamed",
            ])]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("durable filtered stream should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 2);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1], vec![3]]);
}

pub fn durable_replay_respects_payload_predicate_filters<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new("account-opened", json!({ "accountId": "a2" })),
        ])
        .expect("first historical append should succeed");
    store
        .append(vec![
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Primary" }),
            ),
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a2", "name": "Secondary" }),
            ),
        ])
        .expect("second historical append should succeed");

    let _stream = store
        .stream_to_durable(
            &DurableStream::new("account-a1"),
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("durable filtered stream should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 2);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1], vec![3]]);
}

pub fn durable_replay_uses_shared_filter_or_and_semantics<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new("account-opened", json!({ "accountId": "b9" })),
            NewEvent::new("account-renamed", json!({ "accountId": "b9" })),
            NewEvent::new(
                "account-credited",
                json!({ "accountId": "a1", "amount": 10 }),
            ),
        ])
        .expect("historical append should succeed");

    let _stream = store
        .stream_to_durable(
            &DurableStream::new("shared-filter-semantics"),
            &EventQuery::all().with_filters([
                EventFilter::for_event_types(["account-opened"])
                    .with_payload_predicates([json!({ "accountId": "a1" })]),
                EventFilter::for_event_types(["account-renamed"]),
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("durable filtered stream should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1, 3]]);
}

pub fn durable_replay_to_live_boundary_has_no_duplicates_or_gaps<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");

    let _stream = store
        .stream_all_durable(
            &DurableStream::new("transition-stream"),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("durable stream should succeed");

    let replay_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(batch_sequences(&replay_batches), vec![vec![1]]);

    store
        .append(vec![NewEvent::new(
            "account-renamed",
            json!({ "accountId": "a1", "name": "Primary" }),
        )])
        .expect("live append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 2);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1], vec![2]]);
}

pub fn durable_stream_replay_failure_does_not_advance_cursor_and_retry_replays_from_same_position<
    S,
    F,
>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let recovery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");

    let error = match store.stream_all_durable(
        &DurableStream::new("failing-replay-stream"),
        Arc::new(|_| Err(StreamHandlerError::new("expected replay failure"))),
    ) {
        Ok(_) => panic!("failing replay should return an error"),
        Err(error) => error,
    };
    assert!(matches!(error, EventStoreError::BackendFailure { .. }));

    let query_result = store
        .query(&EventQuery::all())
        .expect("historical commit should remain queryable after replay failure");
    assert_eq!(query_result.event_records.len(), 1);

    let _recovered_stream = store
        .stream_all_durable(
            &DurableStream::new("failing-replay-stream"),
            recording_handle(Arc::clone(&recovery_log)),
        )
        .expect("durable stream should be resumable after replay failure");

    let delivered_batches = wait_for_delivery_count(&recovery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1]]);
}

pub fn durable_replay_panic_does_not_advance_cursor_and_retry_replays_from_same_position<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let recovery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");

    let error = match store.stream_all_durable(
        &DurableStream::new("panicking-replay-stream"),
        Arc::new(|_| -> Result<(), StreamHandlerError> { panic!("expected replay panic") }),
    ) {
        Ok(_) => panic!("panicking replay should return an error"),
        Err(error) => error,
    };
    assert!(matches!(error, EventStoreError::BackendFailure { .. }));

    let query_result = store
        .query(&EventQuery::all())
        .expect("historical commit should remain queryable after replay panic");
    assert_eq!(query_result.event_records.len(), 1);

    let _recovered_stream = store
        .stream_all_durable(
            &DurableStream::new("panicking-replay-stream"),
            recording_handle(Arc::clone(&recovery_log)),
        )
        .expect("durable stream should be resumable after replay panic");

    let delivered_batches = wait_for_delivery_count(&recovery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1]]);
}

pub fn durable_live_failure_does_not_roll_back_append_success_or_advance_cursor<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let recovery_log = Arc::new(Mutex::new(Vec::new()));

    let _failed_stream = store
        .stream_all_durable(
            &DurableStream::new("failing-live-stream"),
            Arc::new(|_| Err(StreamHandlerError::new("expected live delivery failure"))),
        )
        .expect("durable stream should register");

    let append_result = store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should still succeed");
    assert_eq!(append_result.first_sequence_number, 1);

    let query_result = store
        .query(&EventQuery::all())
        .expect("committed facts should remain queryable after handler failure");
    assert_eq!(query_result.event_records.len(), 1);

    let _recovered_stream = wait_for_durable_resubscribe_after_failure(
        &store,
        "failing-live-stream",
        Arc::clone(&recovery_log),
    );

    let delivered_batches = wait_for_delivery_count(&recovery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1]]);
}

pub fn durable_unsubscribe_stops_future_live_delivery_but_retains_durable_cursor_state<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let first_delivery_log = Arc::new(Mutex::new(Vec::new()));

    let first_stream = store
        .stream_all_durable(
            &DurableStream::new("unsubscribe-stream"),
            recording_handle(Arc::clone(&first_delivery_log)),
        )
        .expect("durable stream should succeed");

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("first append should succeed");

    let first_batches = wait_for_delivery_count(&first_delivery_log, 1);
    assert_eq!(batch_sequences(&first_batches), vec![vec![1]]);

    first_stream.unsubscribe();

    store
        .append(vec![NewEvent::new(
            "account-renamed",
            json!({ "accountId": "a1", "name": "Primary" }),
        )])
        .expect("second append should succeed");

    assert_delivery_count_stays(&first_delivery_log, 1);

    let resumed_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _resumed_stream = store
        .stream_all_durable(
            &DurableStream::new("unsubscribe-stream"),
            recording_handle(Arc::clone(&resumed_delivery_log)),
        )
        .expect("durable resubscribe should succeed");

    let resumed_batches = wait_for_delivery_count(&resumed_delivery_log, 1);
    assert_eq!(batch_sequences(&resumed_batches), vec![vec![2]]);
}

fn wait_for_durable_resubscribe_after_failure<S>(
    store: &S,
    durable_stream_id: &str,
    delivery_log: Arc<Mutex<Vec<Vec<EventRecord>>>>,
) -> factstore::EventStream
where
    S: EventStore,
{
    let deadline = Instant::now() + Duration::from_secs(1);

    loop {
        match store.stream_all_durable(
            &DurableStream::new(durable_stream_id),
            recording_handle(Arc::clone(&delivery_log)),
        ) {
            Ok(stream) => return stream,
            Err(EventStoreError::BackendFailure { .. }) => {
                assert!(
                    Instant::now() < deadline,
                    "durable stream {durable_stream_id} should be cleaned up after failed delivery"
                );
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!(
                "durable stream {durable_stream_id} should be resumable after failed delivery: {error:?}"
            ),
        }
    }
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

fn assert_no_delivery(delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>) {
    assert_delivery_count_stays(delivery_log, 0);
}

fn assert_delivery_count_stays(
    delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>,
    expected_count: usize,
) {
    let deadline = Instant::now() + Duration::from_millis(100);

    while Instant::now() < deadline {
        let delivered_batches = delivered_batches(delivery_log);
        assert_eq!(
            delivered_batches.len(),
            expected_count,
            "expected {expected_count} delivered batches, got {}",
            delivered_batches.len()
        );
        thread::sleep(Duration::from_millis(5));
    }
}
