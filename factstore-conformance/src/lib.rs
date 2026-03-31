use factstore::{
    EventFilter, EventQuery, EventRecord, EventStore, EventStoreError, HandleEvents, NewEvent,
    SubscriptionHandlerError,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn new_event(event_type: &str, payload: Value) -> NewEvent {
    NewEvent::new(event_type, payload)
}

fn recording_handle(delivery_log: Arc<Mutex<Vec<Vec<EventRecord>>>>) -> HandleEvents {
    Arc::new(move |event_records| {
        delivery_log
            .lock()
            .expect("delivery log lock should succeed")
            .push(event_records);
        Ok(())
    })
}

fn failing_handle() -> HandleEvents {
    Arc::new(|_| {
        Err(SubscriptionHandlerError::new(
            "expected test handler failure",
        ))
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
        let delivered_batches = delivered_batches(delivery_log);
        assert!(
            delivered_batches.is_empty(),
            "expected no delivered batches, got {}",
            delivered_batches.len()
        );
        thread::sleep(Duration::from_millis(5));
    }
}

pub fn append_assigns_consecutive_global_sequence_numbers<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    let append_result = store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    assert_eq!(append_result.first_sequence_number, 1);
    assert_eq!(append_result.last_sequence_number, 2);
    assert_eq!(append_result.committed_count, 2);

    let second_append_result = store
        .append(vec![new_event(
            "account-closed",
            json!({ "accountId": "a1" }),
        )])
        .expect("second append should succeed");

    assert_eq!(second_append_result.first_sequence_number, 3);
    assert_eq!(second_append_result.last_sequence_number, 3);
    assert_eq!(second_append_result.committed_count, 1);
}

pub fn empty_append_input_returns_typed_error<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    let error = store
        .append(Vec::new())
        .expect_err("empty append should fail");

    assert_eq!(error, EventStoreError::EmptyAppend);
}

pub fn subscribe_all_callback_receives_a_future_committed_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");

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

pub fn subscribe_does_not_replay_historical_events<S, F>(create_store: F)
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
        .subscribe_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");

    assert!(delivered_batches(&delivery_log).is_empty());
}

pub fn two_subscribers_receive_the_same_committed_batches<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let first_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let second_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _first_subscription = store
        .subscribe_all(recording_handle(Arc::clone(&first_delivery_log)))
        .expect("first subscribe_all should succeed");
    let _second_subscription = store
        .subscribe_all(recording_handle(Arc::clone(&second_delivery_log)))
        .expect("second subscribe_all should succeed");

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

pub fn subscription_batches_arrive_in_commit_order<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");

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
        .subscribe_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");
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
        EventStoreError::ConditionalAppendConflict {
            expected: None,
            actual: Some(1),
        }
    );
    assert_no_delivery(&delivery_log);
}

pub fn unsubscribing_one_subscriber_does_not_break_delivery_for_others<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let dropped_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let active_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let dropped_subscription = store
        .subscribe_all(recording_handle(Arc::clone(&dropped_delivery_log)))
        .expect("subscribe_all should succeed");
    let _active_subscription = store
        .subscribe_all(recording_handle(Arc::clone(&active_delivery_log)))
        .expect("subscribe_all should succeed");

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

pub fn subscription_delivery_preserves_the_committed_batch_shape<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_all(recording_handle(Arc::clone(&delivery_log)))
        .expect("subscribe_all should succeed");

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

pub fn filtered_subscription_with_event_type_receives_only_matching_future_events<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("subscribe_to should succeed");

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

pub fn filtered_subscription_with_payload_predicate_receives_only_matching_future_events<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("subscribe_to should succeed");

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

pub fn filtered_subscription_non_matching_commit_produces_no_delivery<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("subscribe_to should succeed");

    store
        .append(vec![new_event(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    assert_no_delivery(&delivery_log);
}

pub fn filtered_subscription_mixed_committed_batch_yields_one_filtered_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("subscribe_to should succeed");

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

pub fn filtered_subscription_preserves_event_order_inside_delivered_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _subscription = store
        .subscribe_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("subscribe_to should succeed");

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

pub fn append_if_conflict_emits_no_filtered_delivery<S, F>(create_store: F)
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
        .subscribe_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
            ]),
            recording_handle(Arc::clone(&delivery_log)),
        )
        .expect("subscribe_to should succeed");
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
        EventStoreError::ConditionalAppendConflict {
            expected: None,
            actual: Some(1),
        }
    );
    assert_no_delivery(&delivery_log);
}

pub fn differently_filtered_subscribers_observe_the_same_commit_differently<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();
    let event_type_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let payload_delivery_log = Arc::new(Mutex::new(Vec::new()));
    let _event_type_subscription = store
        .subscribe_to(
            &EventQuery::for_event_types(["account-opened"]),
            recording_handle(Arc::clone(&event_type_delivery_log)),
        )
        .expect("subscribe_to should succeed");
    let _payload_subscription = store
        .subscribe_to(
            &EventQuery::all().with_filters([
                EventFilter::default().with_payload_predicates([json!({ "accountId": "a2" })])
            ]),
            recording_handle(Arc::clone(&payload_delivery_log)),
        )
        .expect("subscribe_to should succeed");

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
        .subscribe_all(failing_handle())
        .expect("subscribe_all should succeed");
    let _successful_subscription = store
        .subscribe_all(recording_handle(Arc::clone(&successful_delivery_log)))
        .expect("subscribe_all should succeed");

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

pub fn query_returns_events_in_ascending_order<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
            new_event("account-debited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 3);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 2);
    assert_eq!(query_result.event_records[2].sequence_number, 3);
    assert_eq!(query_result.last_returned_sequence_number, Some(3));
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn query_with_min_sequence_number_only_returns_events_after_that_sequence<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
            new_event("account-debited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_min_sequence_number(2))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.event_records[0].sequence_number, 3);
    assert_eq!(query_result.last_returned_sequence_number, Some(3));
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn query_against_an_empty_store_returns_explicit_empty_result<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed");

    assert!(query_result.event_records.is_empty());
    assert_eq!(query_result.last_returned_sequence_number, None);
    assert_eq!(query_result.current_context_version, None);
}

pub fn current_context_version_for_payload_filtered_queries_uses_the_full_matching_context<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
            new_event(
                "account-opened",
                json!({ "accountId": "a1", "name": "Rico" }),
            ),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.last_returned_sequence_number, Some(3));
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn min_sequence_number_does_not_narrow_current_context_version_for_payload_filtered_queries<
    S,
    F,
>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
            new_event("account-opened", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(
            &EventQuery::all()
                .with_filters([
                    EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
                ])
                .with_min_sequence_number(3),
        )
        .expect("query should succeed");

    assert!(query_result.event_records.is_empty());
    assert_eq!(query_result.last_returned_sequence_number, None);
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn all_events_query_and_filtered_query_report_their_own_context_versions<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a2" })),
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-debited", json!({ "accountId": "a3" })),
        ])
        .expect("append should succeed");

    let all_events_query_result = store
        .query(&EventQuery::all().with_min_sequence_number(3))
        .expect("query should succeed");
    let filtered_query_result = store
        .query(&EventQuery::for_event_types(["account-opened"]).with_min_sequence_number(3))
        .expect("query should succeed");

    assert_eq!(all_events_query_result.event_records.len(), 1);
    assert_eq!(
        all_events_query_result.last_returned_sequence_number,
        Some(4)
    );
    assert_eq!(all_events_query_result.current_context_version, Some(4));

    assert!(filtered_query_result.event_records.is_empty());
    assert_eq!(filtered_query_result.last_returned_sequence_number, None);
    assert_eq!(filtered_query_result.current_context_version, Some(3));
}

pub fn or_across_filters_matches_any_filter<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a2" })),
            new_event("account-debited", json!({ "accountId": "a3" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::for_event_types(["account-opened"]),
            EventFilter::for_event_types(["account-debited"]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 3);
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn or_across_event_types_inside_one_filter_matches_any_event_type<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
            new_event("account-debited", json!({ "accountId": "a1" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(
            &EventQuery::all().with_filters([EventFilter::for_event_types([
                "account-opened",
                "account-debited",
            ])]),
        )
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 3);
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn or_across_payload_predicates_inside_one_filter_matches_any_payload_predicate<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
            new_event("account-opened", json!({ "accountId": "a3" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(
            &EventQuery::all().with_filters([EventFilter::default().with_payload_predicates([
                json!({ "accountId": "a1" }),
                json!({ "accountId": "a3" }),
            ])]),
        )
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 3);
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn and_between_event_type_and_payload_predicate_within_one_filter<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-credited", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
        ])
        .expect("append should succeed");

    let query_result = store
        .query(
            &EventQuery::all().with_filters([EventFilter::for_event_types(["account-opened"])
                .with_payload_predicates([json!({ "accountId": "a2" })])]),
        )
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.event_records[0].sequence_number, 3);
    assert_eq!(query_result.current_context_version, Some(3));
}

pub fn scalar_subset_match_works<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1", "name": "Rico" }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn nested_object_subset_match_works<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "user-created",
            json!({ "user": { "id": "u1", "role": "admin" } }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "user": { "id": "u1" } })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn array_subset_match_with_scalar_elements_works<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "customer-tagged",
            json!({ "tags": ["vip", "beta"] }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "tags": ["vip"] })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn array_subset_match_with_object_elements_works<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "order-created",
            json!({ "items": [{ "sku": "a", "qty": 2 }, { "sku": "b" }] }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "items": [{ "sku": "a" }] })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn payload_predicate_no_match_returns_no_events<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1", "name": "Rico" }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "accountId": "missing" })]),
        ]))
        .expect("query should succeed");

    assert!(query_result.event_records.is_empty());
    assert_eq!(query_result.last_returned_sequence_number, None);
    assert_eq!(query_result.current_context_version, None);
}

pub fn empty_event_types_filter_returns_no_events_and_no_context_version<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([EventFilter {
            event_types: Some(Vec::new()),
            payload_predicates: None,
        }]))
        .expect("query should succeed");

    assert!(query_result.event_records.is_empty());
    assert_eq!(query_result.last_returned_sequence_number, None);
    assert_eq!(query_result.current_context_version, None);
}

pub fn conditional_append_uses_empty_event_types_filter_as_empty_context<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let context_query = EventQuery::all().with_filters([EventFilter {
        event_types: Some(Vec::new()),
        payload_predicates: None,
    }]);

    let append_result = store
        .append_if(
            vec![new_event("account-renamed", json!({ "accountId": "a2" }))],
            &context_query,
            None,
        )
        .expect("conditional append should succeed");

    assert_eq!(append_result.first_sequence_number, 2);
    assert_eq!(append_result.last_sequence_number, 2);
    assert_eq!(append_result.committed_count, 1);
}

pub fn payload_array_match_is_order_insensitive<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "customer-tagged",
            json!({ "tags": ["vip", "beta"] }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "tags": ["beta", "vip"] })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn payload_array_object_match_can_match_non_first_payload_element<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "order-created",
            json!({ "items": [{ "sku": "a" }, { "sku": "b", "qty": 2 }] }),
        )])
        .expect("append should succeed");

    let query_result = store
        .query(&EventQuery::all().with_filters([
            EventFilter::default().with_payload_predicates([json!({ "items": [{ "sku": "b" }] })]),
        ]))
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn conditional_append_succeeds_for_matching_payload_filtered_context_version<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event("account-opened", json!({ "accountId": "a2" })),
        ])
        .expect("append should succeed");

    let context_query = EventQuery::all().with_filters([
        EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
    ]);
    let append_result = store
        .append_if(
            vec![new_event("account-renamed", json!({ "accountId": "a3" }))],
            &context_query,
            Some(1),
        )
        .expect("conditional append should succeed");

    assert_eq!(append_result.first_sequence_number, 3);
    assert_eq!(append_result.last_sequence_number, 3);
    assert_eq!(append_result.committed_count, 1);
}

pub fn conditional_append_fails_for_stale_payload_filtered_context_version<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![
            new_event("account-opened", json!({ "accountId": "a1" })),
            new_event(
                "account-opened",
                json!({ "accountId": "a1", "name": "Rico" }),
            ),
        ])
        .expect("append should succeed");

    let context_query = EventQuery::all().with_filters([
        EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
    ]);
    let error = store
        .append_if(
            vec![new_event("account-renamed", json!({ "accountId": "a3" }))],
            &context_query,
            Some(1),
        )
        .expect_err("conditional append should fail");

    assert_eq!(
        error,
        EventStoreError::ConditionalAppendConflict {
            expected: Some(1),
            actual: Some(2),
        }
    );
}

pub fn failed_conditional_append_does_not_append_any_part_of_the_batch<S, F>(create_store: F)
where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let context_query = EventQuery::all()
        .with_filters([
            EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
        ])
        .with_min_sequence_number(1);

    let _ = store
        .append_if(
            vec![
                new_event("account-renamed", json!({ "accountId": "a1" })),
                new_event("account-credited", json!({ "accountId": "a1" })),
            ],
            &context_query,
            None,
        )
        .expect_err("conditional append should fail");

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 1);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.last_returned_sequence_number, Some(1));
    assert_eq!(query_result.current_context_version, Some(1));
}

pub fn failed_conditional_append_does_not_consume_sequence_numbers_for_later_commits<S, F>(
    create_store: F,
) where
    S: EventStore,
    F: Fn() -> S,
{
    let store = create_store();

    store
        .append(vec![new_event(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("append should succeed");

    let context_query = EventQuery::all().with_filters([
        EventFilter::default().with_payload_predicates([json!({ "accountId": "a1" })])
    ]);

    let conflict = store
        .append_if(
            vec![new_event("account-renamed", json!({ "accountId": "a1" }))],
            &context_query,
            None,
        )
        .expect_err("conditional append should fail");

    assert_eq!(
        conflict,
        EventStoreError::ConditionalAppendConflict {
            expected: None,
            actual: Some(1),
        }
    );

    let append_result = store
        .append(vec![new_event(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("later append should succeed");

    assert_eq!(append_result.first_sequence_number, 2);
    assert_eq!(append_result.last_sequence_number, 2);
    assert_eq!(append_result.committed_count, 1);

    let query_result = store
        .query(&EventQuery::all())
        .expect("query should succeed");

    assert_eq!(query_result.event_records.len(), 2);
    assert_eq!(query_result.event_records[0].sequence_number, 1);
    assert_eq!(query_result.event_records[1].sequence_number, 2);
    assert_eq!(query_result.last_returned_sequence_number, Some(2));
    assert_eq!(query_result.current_context_version, Some(2));
}
