use factstore_conformance as store_conformance;
use factstore_memory::MemoryStore;

#[test]
fn conditional_append_succeeds_for_matching_payload_filtered_context_version() {
    store_conformance::conditional_append_succeeds_for_matching_payload_filtered_context_version(
        MemoryStore::new,
    );
}

#[test]
fn conditional_append_fails_for_stale_payload_filtered_context_version() {
    store_conformance::conditional_append_fails_for_stale_payload_filtered_context_version(
        MemoryStore::new,
    );
}

#[test]
fn failed_conditional_append_does_not_append_any_part_of_the_batch() {
    store_conformance::failed_conditional_append_does_not_append_any_part_of_the_batch(
        MemoryStore::new,
    );
}
