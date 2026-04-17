mod support;

use factstr_conformance as store_conformance;

#[test]
fn conditional_append_succeeds_for_matching_payload_filtered_context_version() {
    support::run_store_test(
        store_conformance::conditional_append_succeeds_for_matching_payload_filtered_context_version,
    );
}

#[test]
fn conditional_append_fails_for_stale_payload_filtered_context_version() {
    support::run_store_test(
        store_conformance::conditional_append_fails_for_stale_payload_filtered_context_version,
    );
}

#[test]
fn failed_conditional_append_does_not_append_any_part_of_the_batch() {
    support::run_store_test(
        store_conformance::failed_conditional_append_does_not_append_any_part_of_the_batch,
    );
}

#[test]
fn failed_conditional_append_does_not_consume_sequence_numbers_for_later_commits() {
    support::run_store_test(
        store_conformance::failed_conditional_append_does_not_consume_sequence_numbers_for_later_commits,
    );
}
