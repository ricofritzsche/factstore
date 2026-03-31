mod support;

use factstore_conformance as store_conformance;

#[test]
fn subscribe_receives_a_future_committed_append_batch() {
    support::run_store_test(store_conformance::subscribe_receives_a_future_committed_append_batch);
}

#[test]
fn subscribe_does_not_replay_historical_events() {
    support::run_store_test(store_conformance::subscribe_does_not_replay_historical_events);
}

#[test]
fn two_subscribers_receive_the_same_committed_batches() {
    support::run_store_test(store_conformance::two_subscribers_receive_the_same_committed_batches);
}

#[test]
fn subscription_batches_arrive_in_commit_order() {
    support::run_store_test(store_conformance::subscription_batches_arrive_in_commit_order);
}

#[test]
fn append_if_conflict_emits_no_subscription_batch() {
    support::run_store_test(store_conformance::append_if_conflict_emits_no_subscription_batch);
}

#[test]
fn dropping_one_subscription_does_not_break_append_for_others() {
    support::run_store_test(
        store_conformance::dropping_one_subscription_does_not_break_append_for_others,
    );
}

#[test]
fn subscription_delivery_preserves_the_committed_batch_shape() {
    support::run_store_test(
        store_conformance::subscription_delivery_preserves_the_committed_batch_shape,
    );
}
