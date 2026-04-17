use factstr::{EventFilter, EventQuery, EventStore};
use serde_json::json;

use crate::support::new_event;

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
