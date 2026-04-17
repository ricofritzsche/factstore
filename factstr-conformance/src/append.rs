use factstr::{EventStore, EventStoreError};
use serde_json::json;

use crate::support::new_event;

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
