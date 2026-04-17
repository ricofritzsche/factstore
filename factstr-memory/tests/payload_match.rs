use factstr_conformance as store_conformance;
use factstr_memory::MemoryStore;

#[test]
fn or_across_filters_matches_any_filter() {
    store_conformance::or_across_filters_matches_any_filter(MemoryStore::new);
}

#[test]
fn or_across_event_types_inside_one_filter_matches_any_event_type() {
    store_conformance::or_across_event_types_inside_one_filter_matches_any_event_type(
        MemoryStore::new,
    );
}

#[test]
fn or_across_payload_predicates_inside_one_filter_matches_any_payload_predicate() {
    store_conformance::or_across_payload_predicates_inside_one_filter_matches_any_payload_predicate(
        MemoryStore::new,
    );
}

#[test]
fn and_between_event_type_and_payload_predicate_within_one_filter() {
    store_conformance::and_between_event_type_and_payload_predicate_within_one_filter(
        MemoryStore::new,
    );
}

#[test]
fn scalar_subset_match_works() {
    store_conformance::scalar_subset_match_works(MemoryStore::new);
}

#[test]
fn nested_object_subset_match_works() {
    store_conformance::nested_object_subset_match_works(MemoryStore::new);
}

#[test]
fn array_subset_match_with_scalar_elements_works() {
    store_conformance::array_subset_match_with_scalar_elements_works(MemoryStore::new);
}

#[test]
fn array_subset_match_with_object_elements_works() {
    store_conformance::array_subset_match_with_object_elements_works(MemoryStore::new);
}

#[test]
fn payload_predicate_no_match_returns_no_events() {
    store_conformance::payload_predicate_no_match_returns_no_events(MemoryStore::new);
}

#[test]
fn empty_event_types_filter_returns_no_events_and_no_context_version() {
    store_conformance::empty_event_types_filter_returns_no_events_and_no_context_version(
        MemoryStore::new,
    );
}

#[test]
fn conditional_append_uses_empty_event_types_filter_as_empty_context() {
    store_conformance::conditional_append_uses_empty_event_types_filter_as_empty_context(
        MemoryStore::new,
    );
}

#[test]
fn payload_array_match_is_order_insensitive() {
    store_conformance::payload_array_match_is_order_insensitive(MemoryStore::new);
}

#[test]
fn payload_array_object_match_can_match_non_first_payload_element() {
    store_conformance::payload_array_object_match_can_match_non_first_payload_element(
        MemoryStore::new,
    );
}
