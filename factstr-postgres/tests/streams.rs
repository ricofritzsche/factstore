mod support;

use factstr_conformance as store_conformance;

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
