use factstore_conformance as store_conformance;
use factstore_memory::MemoryStore;

#[test]
fn subscribe_receives_a_future_committed_append_batch() {
    store_conformance::subscribe_receives_a_future_committed_append_batch(MemoryStore::new);
}

#[test]
fn subscribe_does_not_replay_historical_events() {
    store_conformance::subscribe_does_not_replay_historical_events(MemoryStore::new);
}

#[test]
fn two_subscribers_receive_the_same_committed_batches() {
    store_conformance::two_subscribers_receive_the_same_committed_batches(MemoryStore::new);
}

#[test]
fn subscription_batches_arrive_in_commit_order() {
    store_conformance::subscription_batches_arrive_in_commit_order(MemoryStore::new);
}

#[test]
fn append_if_conflict_emits_no_subscription_batch() {
    store_conformance::append_if_conflict_emits_no_subscription_batch(MemoryStore::new);
}

#[test]
fn dropping_one_subscription_does_not_break_append_for_others() {
    store_conformance::dropping_one_subscription_does_not_break_append_for_others(MemoryStore::new);
}

#[test]
fn subscription_delivery_preserves_the_committed_batch_shape() {
    store_conformance::subscription_delivery_preserves_the_committed_batch_shape(MemoryStore::new);
}
