use factstore_conformance as store_conformance;
use factstore_memory::MemoryStore;

#[test]
fn stream_all_handler_receives_a_future_committed_batch() {
    store_conformance::stream_all_handler_receives_a_future_committed_batch(MemoryStore::new);
}

#[test]
fn stream_does_not_replay_historical_events() {
    store_conformance::stream_does_not_replay_historical_events(MemoryStore::new);
}

#[test]
fn two_streams_receive_the_same_committed_batches() {
    store_conformance::two_streams_receive_the_same_committed_batches(MemoryStore::new);
}

#[test]
fn stream_batches_arrive_in_commit_order() {
    store_conformance::stream_batches_arrive_in_commit_order(MemoryStore::new);
}

#[test]
fn append_if_conflict_emits_no_delivery() {
    store_conformance::append_if_conflict_emits_no_delivery(MemoryStore::new);
}

#[test]
fn unsubscribing_one_stream_does_not_break_delivery_for_others() {
    store_conformance::unsubscribing_one_stream_does_not_break_delivery_for_others(
        MemoryStore::new,
    );
}

#[test]
fn stream_delivery_preserves_the_committed_batch_shape() {
    store_conformance::stream_delivery_preserves_the_committed_batch_shape(MemoryStore::new);
}

#[test]
fn filtered_stream_with_event_type_receives_only_matching_future_events() {
    store_conformance::filtered_stream_with_event_type_receives_only_matching_future_events(
        MemoryStore::new,
    );
}

#[test]
fn filtered_stream_with_payload_predicate_receives_only_matching_future_events() {
    store_conformance::filtered_stream_with_payload_predicate_receives_only_matching_future_events(
        MemoryStore::new,
    );
}

#[test]
fn filtered_stream_non_matching_commit_produces_no_delivery() {
    store_conformance::filtered_stream_non_matching_commit_produces_no_delivery(MemoryStore::new);
}

#[test]
fn filtered_stream_mixed_committed_batch_yields_one_filtered_batch() {
    store_conformance::filtered_stream_mixed_committed_batch_yields_one_filtered_batch(
        MemoryStore::new,
    );
}

#[test]
fn filtered_stream_preserves_event_order_inside_delivered_batch() {
    store_conformance::filtered_stream_preserves_event_order_inside_delivered_batch(
        MemoryStore::new,
    );
}

#[test]
fn append_if_conflict_emits_no_filtered_stream_delivery() {
    store_conformance::append_if_conflict_emits_no_filtered_stream_delivery(MemoryStore::new);
}

#[test]
fn differently_filtered_streams_observe_the_same_commit_differently() {
    store_conformance::differently_filtered_streams_observe_the_same_commit_differently(
        MemoryStore::new,
    );
}

#[test]
fn handler_failure_does_not_roll_back_append_success() {
    store_conformance::handler_failure_does_not_roll_back_append_success(MemoryStore::new);
}
