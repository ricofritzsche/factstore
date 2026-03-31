use factstore::{EventFilter, EventQuery, EventStore, EventStoreError};
use serde_json::json;

use crate::support::new_event;

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
