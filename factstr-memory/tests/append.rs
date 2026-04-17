use factstr_conformance as store_conformance;
use factstr_memory::MemoryStore;

#[test]
fn append_assigns_consecutive_global_sequence_numbers() {
    store_conformance::append_assigns_consecutive_global_sequence_numbers(MemoryStore::new);
}

#[test]
fn empty_append_input_returns_typed_error() {
    store_conformance::empty_append_input_returns_typed_error(MemoryStore::new);
}
