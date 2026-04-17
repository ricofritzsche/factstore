mod support;

use factstr_conformance as store_conformance;

#[test]
fn append_assigns_consecutive_global_sequence_numbers() {
    support::run_store_test(store_conformance::append_assigns_consecutive_global_sequence_numbers);
}

#[test]
fn empty_append_input_returns_typed_error() {
    support::run_store_test(store_conformance::empty_append_input_returns_typed_error);
}
