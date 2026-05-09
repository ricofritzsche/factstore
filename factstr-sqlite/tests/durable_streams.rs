mod support;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use factstr::EventStoreError;
use factstr::{DurableStream, EventFilter, EventQuery, EventRecord, EventStore, NewEvent};
use factstr_conformance as store_conformance;
use factstr_sqlite::SqliteStore;
use serde_json::json;

use support::{
    TemporaryDatabaseFile, append_batch_rows, connect, delete_metadata_key, metadata_value,
    subscriber_cursor,
};

const APPEND_BATCH_BOUNDARY_FORMAT_KEY: &str = "append_batch_boundary_format";
const APPEND_BATCH_BOUNDARY_FORMAT_SPARSE_V1: &str = "sparse_v1";

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
fn durable_replay_waits_for_handler_completion_before_advancing_cursor() {
    support::run_store_test(
        store_conformance::durable_replay_waits_for_handler_completion_before_advancing_cursor,
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
    assert_cursor_reaches(database_file.path(), "account-directory", 2);

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
    assert_cursor_reaches(database_file.path(), "account-directory", 3);
}

#[test]
fn old_style_single_event_append_batch_rows_do_not_break_replay() {
    let database_file = TemporaryDatabaseFile::new("durable-subscriptions-old-single-boundary");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let recovery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");
    insert_append_batch(database_file.path(), 1, 1);

    let _subscription = store
        .stream_all_durable(
            &DurableStream::new("setup-failure"),
            recording_handle(Arc::clone(&recovery_log)),
        )
        .expect("durable replay should accept old-style single-event boundaries");

    let delivered_batches = wait_for_delivery_count(&recovery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1]]);
    assert_cursor_reaches(database_file.path(), "setup-failure", 1);
}

#[test]
fn durable_replay_is_rejected_when_sparse_boundary_marker_is_missing() {
    let database_file = TemporaryDatabaseFile::new("missing-sparse-boundary-marker");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("historical append should succeed");

    delete_append_batch_boundary_format(database_file.path());

    let error = match store.stream_all_durable(
        &DurableStream::new("missing-marker"),
        factstr::HandleStream::new(|_| async { Ok(()) }),
    ) {
        Ok(_) => panic!("durable replay should reject a missing sparse boundary marker"),
        Err(error) => error,
    };

    match error {
        EventStoreError::BackendFailure { message } => {
            assert!(
                message.contains(APPEND_BATCH_BOUNDARY_FORMAT_KEY),
                "expected sparse boundary marker error, got: {message}"
            );
        }
        other => panic!("expected backend failure, got {other:?}"),
    }
}

#[test]
fn single_event_appends_do_not_create_append_batch_rows() {
    let database_file = TemporaryDatabaseFile::new("single-event-append-batches");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("first single-event append should succeed");
    store
        .append(vec![NewEvent::new(
            "account-renamed",
            json!({ "accountId": "a1", "name": "Primary" }),
        )])
        .expect("second single-event append should succeed");

    assert_eq!(
        read_append_batch_rows(database_file.path()),
        Vec::<(u64, u64)>::new()
    );
}

#[test]
fn multi_event_append_creates_one_append_batch_row() {
    let database_file = TemporaryDatabaseFile::new("multi-event-append-batches");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");

    store
        .append(vec![
            NewEvent::new("account-opened", json!({ "accountId": "a1" })),
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Primary" }),
            ),
            NewEvent::new(
                "account-credited",
                json!({ "accountId": "a1", "amount": 50 }),
            ),
        ])
        .expect("multi-event append should succeed");

    assert_eq!(read_append_batch_rows(database_file.path()), vec![(1, 3)]);
}

#[test]
fn durable_replay_uses_sparse_append_batch_boundaries() {
    let database_file = TemporaryDatabaseFile::new("sparse-append-batch-replay");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("single-event append should succeed");
    store
        .append(vec![
            NewEvent::new(
                "account-renamed",
                json!({ "accountId": "a1", "name": "Primary" }),
            ),
            NewEvent::new(
                "account-credited",
                json!({ "accountId": "a1", "amount": 50 }),
            ),
            NewEvent::new(
                "account-debited",
                json!({ "accountId": "a1", "amount": 20 }),
            ),
        ])
        .expect("multi-event append should succeed");
    store
        .append(vec![NewEvent::new(
            "account-closed",
            json!({ "accountId": "a1" }),
        )])
        .expect("trailing single-event append should succeed");

    let _subscription = store
        .stream_all_durable(
            &DurableStream::new("sparse-replay"),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("durable replay should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 3);
    assert_eq!(
        batch_sequences(&delivered_batches),
        vec![vec![1], vec![2, 3, 4], vec![5]]
    );
    assert_cursor_reaches(database_file.path(), "sparse-replay", 5);
}

#[test]
fn filtered_durable_replay_preserves_multi_event_batch_boundary_and_cursor_target() {
    let database_file = TemporaryDatabaseFile::new("filtered-multi-event-replay");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![
            NewEvent::new("inventory-adjusted", json!({ "sku": "a1", "delta": 5 })),
            NewEvent::new("inventory-snapshotted", json!({ "sku": "a1", "count": 5 })),
            NewEvent::new("inventory-adjusted", json!({ "sku": "a1", "delta": -2 })),
        ])
        .expect("multi-event append should succeed");

    let _subscription = store
        .stream_to_durable(
            &DurableStream::new("filtered-multi-event"),
            &EventQuery::all().with_filters([EventFilter::for_event_types(["inventory-adjusted"])]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("filtered durable replay should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(batch_sequences(&delivered_batches), vec![vec![1, 3]]);
    assert_cursor_reaches(database_file.path(), "filtered-multi-event", 3);
}

#[test]
fn filtered_durable_replay_skips_non_matching_multi_event_batches_and_reaches_live_delivery() {
    let database_file = TemporaryDatabaseFile::new("filtered-non-matching-multi-event");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![
            NewEvent::new("inventory-snapshotted", json!({ "sku": "a1", "count": 5 })),
            NewEvent::new("inventory-snapshotted", json!({ "sku": "a1", "count": 7 })),
        ])
        .expect("non-matching multi-event append should succeed");
    store
        .append(vec![NewEvent::new(
            "inventory-adjusted",
            json!({ "sku": "a1", "delta": 2 }),
        )])
        .expect("matching replay append should succeed");

    let _subscription = store
        .stream_to_durable(
            &DurableStream::new("filtered-non-matching"),
            &EventQuery::all().with_filters([EventFilter::for_event_types(["inventory-adjusted"])]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("filtered durable replay should succeed");

    let first_delivery = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(batch_sequences(&first_delivery), vec![vec![3]]);
    assert_cursor_reaches(database_file.path(), "filtered-non-matching", 3);

    store
        .append(vec![NewEvent::new(
            "inventory-adjusted",
            json!({ "sku": "a1", "delta": -1 }),
        )])
        .expect("live matching append should succeed");

    let live_delivery = wait_for_delivery_count(&delivery_log, 2);
    assert_eq!(batch_sequences(&live_delivery), vec![vec![3], vec![4]]);
    assert_cursor_reaches(database_file.path(), "filtered-non-matching", 4);
}

fn recording_handle(delivery_log: Arc<Mutex<Vec<Vec<EventRecord>>>>) -> factstr::HandleStream {
    factstr::HandleStream::new(move |event_records| {
        let delivery_log = Arc::clone(&delivery_log);

        async move {
            delivery_log
                .lock()
                .expect("delivery log lock should succeed")
                .push(event_records);
            Ok(())
        }
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

fn assert_cursor_reaches(database_path: &std::path::Path, subscriber_id: &str, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(1);

    loop {
        let (_, actual) = read_cursor(database_path, subscriber_id);
        if actual == expected {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "expected subscriber cursor {subscriber_id} to reach {expected}, got {actual}"
        );

        thread::sleep(Duration::from_millis(10));
    }
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

fn read_append_batch_rows(database_path: &std::path::Path) -> Vec<(u64, u64)> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_path).await;
        append_batch_rows(&mut connection).await
    })
}

fn delete_append_batch_boundary_format(database_path: &std::path::Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build");

    runtime.block_on(async {
        let mut connection = connect(database_path).await;
        let marker_before = metadata_value(&mut connection, APPEND_BATCH_BOUNDARY_FORMAT_KEY).await;
        assert_eq!(
            marker_before.as_deref(),
            Some(APPEND_BATCH_BOUNDARY_FORMAT_SPARSE_V1)
        );
        delete_metadata_key(&mut connection, APPEND_BATCH_BOUNDARY_FORMAT_KEY).await;
    });
}
