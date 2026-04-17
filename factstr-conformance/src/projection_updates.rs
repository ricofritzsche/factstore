//! Reusable conformance for future committed stream delivery.
//!
//! Durable-stream replay/catch-up has its own reusable conformance in
//! `durable_streams.rs`. This module remains focused on future committed stream
//! delivery.

use factstr::{EventFilter, EventQuery, EventStore};
use serde_json::json;
use std::sync::{Arc, Mutex};

use crate::support::{
    assert_no_delivery, delivered_batches, failing_handle, new_event, recording_handle,
    wait_for_delivery_count,
};

pub fn stream_all_handler_receives_a_future_committed_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("stream_all should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    assert_eq!(delivered_batches[0].len(), 2);
    assert_eq!(delivered_batches[0][0].sequence_number, 1);
    assert_eq!(delivered_batches[0][1].sequence_number, 2);
}

pub fn stream_does_not_replay_historical_events<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("stream_all should succeed");

    assert!(delivered_batches(&delivery_log).is_empty());
}

pub fn two_streams_receive_the_same_committed_batches<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let first_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let second_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _first_subscription = store
        .stream_all(recording_handle(Arc::clone(&first_delivery_log)))
        .expect("first stream_all should succeed");
    let _second_subscription = store
        .stream_all(recording_handle(Arc::clone(&second_delivery_log)))
        .expect("second stream_all should succeed");

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    assert_eq!(
        wait_for_delivery_count(&first_delivery_log, 1),
        wait_for_delivery_count(&second_delivery_log, 1)
    );
}

pub fn stream_batches_arrive_in_commit_order<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("stream_all should succeed");

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("first append should succeed");
    store
        .append(vec![new_event(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("second append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 2);
    assert_eq!(delivered_batches.len(), 2);
    assert_eq!(delivered_batches[0][0].sequence_number, 1);
    assert_eq!(delivered_batches[1][0].sequence_number, 2);
}

pub fn append_if_conflict_emits_no_delivery<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("stream_all should succeed");
    let context_query = EventQuery::all().with_filters([
        EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
    ]);

    let error = store
        .append_if(
            vec![new_event("account-credited", json!({ "accountId": "a1" }))],
            &context_query,
            None,
        )
        .expect_err("conditional append should fail");

    assert_eq!(
        error,
        factstr::EventStoreError::ConditionalAppendConflict {
            expected: None,
            actual: Some(1),
        }
    );
    assert_no_delivery(&delivery_log);
}

pub fn unsubscribing_one_stream_does_not_break_delivery_for_others<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let dropped_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let active_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let dropped_subscription = store
        .stream_all(recording_handle(Arc::clone(&dropped_delivery_log)))
        .expect("stream_all should succeed");
    let _active_subscription = store
        .stream_all(recording_handle(Arc::clone(&active_delivery_log)))
        .expect("stream_all should succeed");

    dropped_subscription.unsubscribe();

    let append_result = store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should still succeed");

    assert_eq!(append_result.first_sequence_number, 1);
    assert_eq!(append_result.last_sequence_number, 1);
    assert_no_delivery(&dropped_delivery_log);
    assert_eq!(wait_for_delivery_count(&active_delivery_log, 1).len(), 1);
}

pub fn stream_delivery_preserves_the_committed_batch_shape<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("stream_all should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    assert_eq!(delivered_batches[0].len(), 2);
    assert_eq!(delivered_batches[0][0].sequence_number, 1);
    assert_eq!(delivered_batches[0][1].sequence_number, 2);
}

pub fn filtered_stream_with_event_type_receives_only_matching_future_events<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("stream_to should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
        ])
        .expect("append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    let committed_batch = &delivered_batches[0];

    assert_eq!(committed_batch.len(), 2);
    assert_eq!(committed_batch[0].sequence_number, 1);
    assert_eq!(committed_batch[1].sequence_number, 3);
    assert!(
        committed_batch
            .iter()
            .all(|event_record| event_record.event_type == "account-opened")
    );
}

pub fn filtered_stream_with_payload_predicate_receives_only_matching_future_events<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("stream_to should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
            new_event("account-renamed", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    let committed_batch = &delivered_batches[0];

    assert_eq!(committed_batch.len(), 2);
    assert_eq!(committed_batch[0].sequence_number, 1);
    assert_eq!(committed_batch[1].sequence_number, 3);
}

pub fn filtered_stream_non_matching_commit_produces_no_delivery<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("stream_to should succeed");

    store
        .append(vec![new_event(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    assert_no_delivery(&delivery_log);
}

pub fn filtered_stream_mixed_committed_batch_yields_one_filtered_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("stream_to should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
        ])
        .expect("append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    let committed_batch = &delivered_batches[0];

    assert_eq!(committed_batch.len(), 2);
    assert_eq!(committed_batch[0].sequence_number, 1);
    assert_eq!(committed_batch[1].sequence_number, 3);
}

pub fn filtered_stream_preserves_event_order_inside_delivered_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .stream_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("stream_to should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
            new_event("account-renamed", json!({ "accountId": "a1" })),
            new_event("account-closed", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let delivered_batches = wait_for_delivery_count(&delivery_log, 1);
    assert_eq!(delivered_batches.len(), 1);
    let committed_batch = &delivered_batches[0];

    assert_eq!(committed_batch.len(), 3);
    assert_eq!(
        committed_batch
            .iter()
            .map(|event_record| event_record.sequence_number)
            .collect::<Vec<_>>(),
        vec![1, 3, 4]
    );
}

pub fn append_if_conflict_emits_no_filtered_stream_delivery<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let _subscription = store
        .stream_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("stream_to should succeed");
    let context_query = EventQuery::all().with_filters([
        EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
    ]);

    let error = store
        .append_if(
            vec![new_event("account-credited", json!({ "accountId": "a1" }))],
            &context_query,
            None,
        )
        .expect_err("conditional append should fail");

    assert_eq!(
        error,
        factstr::EventStoreError::ConditionalAppendConflict {
            expected: None,
            actual: Some(1),
        }
    );
    assert_no_delivery(&delivery_log);
}

pub fn differently_filtered_streams_observe_the_same_commit_differently<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let event_type_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let payload_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _event_type_subscription = store
        .stream_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&event_type_delivery_log)),
        )
        .expect("stream_to should succeed");
    let _payload_subscription = store
        .stream_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a2" })])
            ]),
            recording_handle(Arc::clone(&payload_delivery_log)),
        )
        .expect("stream_to should succeed");

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a2" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
        ])
        .expect("append should succeed");

    let event_type_batches = wait_for_delivery_count(&event_type_delivery_log, 1);
    let payload_batches = wait_for_delivery_count(&payload_delivery_log, 1);
    assert_eq!(event_type_batches.len(), 1);
    assert_eq!(payload_batches.len(), 1);

    assert_eq!(
        event_type_batches[0]
            .iter()
            .map(|event_record| event_record.sequence_number)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(
        payload_batches[0]
            .iter()
            .map(|event_record| event_record.sequence_number)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}

pub fn handler_failure_does_not_roll_back_append_success<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let successful_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _failing_subscription = store
        .stream_all(failing_handle())
        .expect("stream_all should succeed");
    let _successful_subscription = store
        .stream_all(recording_handle(Arc::clone(&successful_delivery_log)))
        .expect("stream_all should succeed");

    let append_result = store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should still succeed");

    assert_eq!(append_result.first_sequence_number, 1);
    assert_eq!(append_result.last_sequence_number, 1);

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed");
    assert_eq!(query_result.event_records.len(), 1);

    let successful_delivery_log = wait_for_delivery_count(&successful_delivery_log, 1);
    assert_eq!(successful_delivery_log.len(), 1);
    assert_eq!(successful_delivery_log[0].len(), 1);
    assert_eq!(successful_delivery_log[0][0].sequence_number, 1);
}
