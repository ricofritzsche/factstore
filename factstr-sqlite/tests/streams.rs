mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use factstr::{EventFilter, EventQuery, EventStore, NewEvent};
use factstr_conformance as store_conformance;
use factstr_sqlite::SqliteStore;
use serde_json::json;

use support::TemporaryDatabaseFile;

#[test]
fn stream_all_handler_receives_a_future_committed_batch() {
    support::run_store_test(
        store_conformance::stream_all_handler_receives_a_future_committed_batch,
    );
}

#[test]
fn stream_does_not_replay_historical_events() {
    support::run_store_test(store_conformance::stream_does_not_replay_historical_events);
}

#[test]
fn two_streams_receive_the_same_committed_batches() {
    support::run_store_test(store_conformance::two_streams_receive_the_same_committed_batches);
}

#[test]
fn stream_batches_arrive_in_commit_order() {
    support::run_store_test(store_conformance::stream_batches_arrive_in_commit_order);
}

#[test]
fn append_if_conflict_emits_no_delivery() {
    support::run_store_test(store_conformance::append_if_conflict_emits_no_delivery);
}

#[test]
fn unsubscribing_one_stream_does_not_break_delivery_for_others() {
    support::run_store_test(
        store_conformance::unsubscribing_one_stream_does_not_break_delivery_for_others,
    );
}

#[test]
fn stream_delivery_preserves_the_committed_batch_shape() {
    support::run_store_test(store_conformance::stream_delivery_preserves_the_committed_batch_shape);
}

#[test]
fn filtered_stream_with_event_type_receives_only_matching_future_events() {
    support::run_store_test(
        store_conformance::filtered_stream_with_event_type_receives_only_matching_future_events,
    );
}

#[test]
fn filtered_stream_with_payload_predicate_receives_only_matching_future_events() {
    support::run_store_test(
        store_conformance::filtered_stream_with_payload_predicate_receives_only_matching_future_events,
    );
}

#[test]
fn filtered_stream_non_matching_commit_produces_no_delivery() {
    support::run_store_test(
        store_conformance::filtered_stream_non_matching_commit_produces_no_delivery,
    );
}

#[test]
fn filtered_stream_mixed_committed_batch_yields_one_filtered_batch() {
    support::run_store_test(
        store_conformance::filtered_stream_mixed_committed_batch_yields_one_filtered_batch,
    );
}

#[test]
fn filtered_stream_preserves_event_order_inside_delivered_batch() {
    support::run_store_test(
        store_conformance::filtered_stream_preserves_event_order_inside_delivered_batch,
    );
}

#[test]
fn append_if_conflict_emits_no_filtered_stream_delivery() {
    support::run_store_test(
        store_conformance::append_if_conflict_emits_no_filtered_stream_delivery,
    );
}

#[test]
fn differently_filtered_streams_observe_the_same_commit_differently() {
    support::run_store_test(
        store_conformance::differently_filtered_streams_observe_the_same_commit_differently,
    );
}

#[test]
fn handler_failure_does_not_roll_back_append_success() {
    support::run_store_test(store_conformance::handler_failure_does_not_roll_back_append_success);
}

#[test]
fn append_if_success_delivers_one_committed_batch() {
    let database_file = TemporaryDatabaseFile::new("subscriptions-append-if-success");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let append_result = store
        .append_if(
            vec![
                NewEvent::new("account-renamed", json!({ "accountId": "a1" })),
                NewEvent::new("account-credited", json!({ "accountId": "a1" })),
            ],
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            Some(1),
        )
        .expect("conditional append should succeed");

    assert_eq!(append_result.first_sequence_number, 2);
    assert_eq!(append_result.last_sequence_number, 3);

    let delivered_batches = wait_for_delivery_count(&delivery_log, 2);
    assert_eq!(delivered_batches.len(), 2);
    assert_eq!(delivered_batches[1].len(), 2);
    assert_eq!(delivered_batches[1][0].sequence_number, 2);
    assert_eq!(delivered_batches[1][1].sequence_number, 3);
}

#[test]
fn empty_append_emits_no_delivery() {
    let database_file = TemporaryDatabaseFile::new("subscriptions-empty-append");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");

    let error = store
        .append(Vec::new())
        .expect_err("empty append should fail");
    assert_eq!(error, factstr::EventStoreError::EmptyAppend);
    assert_no_delivery(&delivery_log);
}

#[test]
fn already_snapshotted_delivery_may_arrive_after_unsubscribe_but_future_commits_do_not() {
    let database_file = TemporaryDatabaseFile::new("subscriptions-unsubscribe-snapshot");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let delivered_batches = Arc::new(Mutex::new(Vec::new()));
    let invocation_count = Arc::new(AtomicUsize::new(0));
    let (first_handler_started_sender, first_handler_started_receiver) = mpsc::channel();
    let release_first_handler = Arc::new((Mutex::new(false), std::sync::Condvar::new()));

    let subscription = store
        .stream_all(Arc::new({
            let delivered_batches = Arc::clone(&delivered_batches);
            let invocation_count = Arc::clone(&invocation_count);
            let release_first_handler = Arc::clone(&release_first_handler);
            move |event_records| {
                delivered_batches
                    .lock()
                    .expect("delivery log lock should succeed")
                    .push(event_records);

                if invocation_count.fetch_add(1, Ordering::SeqCst) == 0 {
                    let _ = first_handler_started_sender.send(());
                    let (lock, condvar) = &*release_first_handler;
                    let mut released = lock.lock().expect("handler gate lock should succeed");
                    while !*released {
                        released = condvar
                            .wait(released)
                            .expect("handler gate wait should succeed");
                    }
                }

                Ok(())
            }
        }))
        .expect("subscribe_all should succeed");

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("first append should succeed");

    first_handler_started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("first delivery should start and block");

    store
        .append(vec![NewEvent::new(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("second append should succeed while the first handler is blocked");

    subscription.unsubscribe();

    store
        .append(vec![NewEvent::new(
            "account-closed",
            json!({ "accountId": "a1" }),
        )])
        .expect("third append should succeed after unsubscribe");

    let (lock, condvar) = &*release_first_handler;
    *lock.lock().expect("handler gate lock should succeed") = true;
    condvar.notify_all();

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let delivered_batches = delivered_batches
            .lock()
            .expect("delivery log lock should succeed")
            .clone();

        if delivered_batches.len() == 2 {
            let delivered_sequences = delivered_batches
                .iter()
                .map(|batch| batch[0].sequence_number)
                .collect::<Vec<_>>();
            assert_eq!(delivered_sequences, vec![1, 2]);
            return;
        }

        assert!(
            Instant::now() < deadline,
            "expected exactly two delivered batches after unsubscribe, got {}",
            delivered_batches.len()
        );

        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn panicking_handler_does_not_roll_back_append_success() {
    let database_file = TemporaryDatabaseFile::new("subscriptions-panic-isolation");
    let store = SqliteStore::open(database_file.path()).expect("sqlite store should open");
    let successful_delivery_log = Arc::new(Mutex::new(Vec::new()));

    let _panicking_subscription = store
        .stream_all(Arc::new(|_| -> Result<(), factstr::StreamHandlerError> {
            panic!("expected sqlite subscription panic")
        }))
        .expect("subscribe_all should succeed");
    let _successful_subscription = store
        .stream_all(recording_handle(Arc::clone(&successful_delivery_log)))
        .expect("subscribe_all should succeed");

    let append_result = store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should still succeed");

    assert_eq!(append_result.first_sequence_number, 1);
    assert_eq!(append_result.last_sequence_number, 1);

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should still succeed after handler panic");
    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.event_records[0].sequence_number, 1);

    let delivered_batches = wait_for_delivery_count(&successful_delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    assert_eq!(delivered_batches[0].len(), 1);
    assert_eq!(delivered_batches[0][0].sequence_number, 1);
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

fn assert_no_delivery(delivery_log: &Arc<Mutex<Vec<Vec<factstr::EventRecord>>>>) {
    let deadline = Instant::now() + Duration::from_millis(100);

    while Instant::now() < deadline {
        let delivered_batches = delivered_batches(delivery_log);
        assert!(
            delivered_batches.is_empty(),
            "expected no delivered batches, got {}",
            delivered_batches.len()
        );
        thread::sleep(Duration::from_millis(5));
    }
}
