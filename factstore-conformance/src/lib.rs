mod append;
mod conditional_append;
mod payload_predicates;
mod projection_updates;
mod query;
mod support;

pub use append::{
    append_assigns_consecutive_global_sequence_numbers, empty_append_input_returns_typed_error,
};
pub use conditional_append::{
    conditional_append_fails_for_stale_payload_filtered_context_version,
    conditional_append_succeeds_for_matching_payload_filtered_context_version,
    failed_conditional_append_does_not_append_any_part_of_the_batch,
    failed_conditional_append_does_not_consume_sequence_numbers_for_later_commits,
};
pub use payload_predicates::{
    and_between_event_type_and_payload_predicate_within_one_filter,
    array_subset_match_with_object_elements_works, array_subset_match_with_scalar_elements_works,
    conditional_append_uses_empty_event_types_filter_as_empty_context,
    empty_event_types_filter_returns_no_events_and_no_context_version,
    nested_object_subset_match_works,
    or_across_event_types_inside_one_filter_matches_any_event_type,
    or_across_filters_matches_any_filter,
    or_across_payload_predicates_inside_one_filter_matches_any_payload_predicate,
    payload_array_match_is_order_insensitive,
    payload_array_object_match_can_match_non_first_payload_element,
    payload_predicate_no_match_returns_no_events, scalar_subset_match_works,
};
pub use projection_updates::{
    append_if_conflict_emits_no_delivery, append_if_conflict_emits_no_filtered_delivery,
    differently_filtered_subscribers_observe_the_same_commit_differently,
    filtered_subscription_mixed_committed_batch_yields_one_filtered_batch,
    filtered_subscription_non_matching_commit_produces_no_delivery,
    filtered_subscription_preserves_event_order_inside_delivered_batch,
    filtered_subscription_with_event_type_receives_only_matching_future_events,
    filtered_subscription_with_payload_predicate_receives_only_matching_future_events,
    handler_failure_does_not_roll_back_append_success,
    subscribe_all_callback_receives_a_future_committed_batch,
    subscribe_does_not_replay_historical_events, subscription_batches_arrive_in_commit_order,
    subscription_delivery_preserves_the_committed_batch_shape,
    two_subscribers_receive_the_same_committed_batches,
    unsubscribing_one_subscriber_does_not_break_delivery_for_others,
};
pub use query::{
    all_events_query_and_filtered_query_report_their_own_context_versions,
    current_context_version_for_payload_filtered_queries_uses_the_full_matching_context,
    min_sequence_number_does_not_narrow_current_context_version_for_payload_filtered_queries,
    query_against_an_empty_store_returns_explicit_empty_result,
    query_returns_events_in_ascending_order,
    query_with_min_sequence_number_only_returns_events_after_that_sequence,
};
