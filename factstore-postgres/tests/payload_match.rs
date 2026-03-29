mod support;

use factstore_conformance as store_conformance;

#[test]
fn or_across_filters_matches_any_filter() {
    support::run_store_test(store_conformance::or_across_filters_matches_any_filter);
}

#[test]
fn or_across_event_types_inside_one_filter_matches_any_event_type() {
    support::run_store_test(
        store_conformance::or_across_event_types_inside_one_filter_matches_any_event_type,
    );
}

#[test]
fn or_across_payload_predicates_inside_one_filter_matches_any_payload_predicate() {
    support::run_store_test(
        store_conformance::or_across_payload_predicates_inside_one_filter_matches_any_payload_predicate,
    );
}

#[test]
fn and_between_event_type_and_payload_predicate_within_one_filter() {
    support::run_store_test(
        store_conformance::and_between_event_type_and_payload_predicate_within_one_filter,
    );
}

#[test]
fn scalar_subset_match_works() {
    support::run_store_test(store_conformance::scalar_subset_match_works);
}

#[test]
fn nested_object_subset_match_works() {
    support::run_store_test(store_conformance::nested_object_subset_match_works);
}

#[test]
fn array_subset_match_with_scalar_elements_works() {
    support::run_store_test(store_conformance::array_subset_match_with_scalar_elements_works);
}

#[test]
fn array_subset_match_with_object_elements_works() {
    support::run_store_test(store_conformance::array_subset_match_with_object_elements_works);
}

#[test]
fn payload_predicate_no_match_returns_no_events() {
    support::run_store_test(store_conformance::payload_predicate_no_match_returns_no_events);
}

#[test]
fn empty_event_types_filter_returns_no_events_and_no_context_version() {
    support::run_store_test(
        store_conformance::empty_event_types_filter_returns_no_events_and_no_context_version,
    );
}

#[test]
fn conditional_append_uses_empty_event_types_filter_as_empty_context() {
    support::run_store_test(
        store_conformance::conditional_append_uses_empty_event_types_filter_as_empty_context,
    );
}

#[test]
fn payload_array_match_is_order_insensitive() {
    support::run_store_test(store_conformance::payload_array_match_is_order_insensitive);
}

#[test]
fn payload_array_object_match_can_match_non_first_payload_element() {
    support::run_store_test(
        store_conformance::payload_array_object_match_can_match_non_first_payload_element,
    );
}
