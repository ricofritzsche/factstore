use std::panic::{AssertUnwindSafe, catch_unwind};

use factstore::{EventQuery, EventRecord, HandleEvents};

use crate::query_match::matches_query;

#[derive(Default)]
pub(crate) struct SubscriptionRegistry {
    next_subscription_id: u64,
    subscribers: Vec<Subscriber>,
}

struct Subscriber {
    id: u64,
    event_query: Option<EventQuery>,
    handle: HandleEvents,
}

pub(crate) struct PendingDelivery {
    subscription_id: u64,
    delivered_batch: Vec<EventRecord>,
    handle: HandleEvents,
}

impl SubscriptionRegistry {
    pub(crate) fn subscribe_all(&mut self, handle: HandleEvents) -> u64 {
        self.subscribe_to(None, handle)
    }

    pub(crate) fn subscribe_to(
        &mut self,
        event_query: Option<EventQuery>,
        handle: HandleEvents,
    ) -> u64 {
        let id = self.next_subscription_id + 1;
        self.next_subscription_id = id;
        self.subscribers.push(Subscriber {
            id,
            event_query,
            handle,
        });
        id
    }

    pub(crate) fn unsubscribe(&mut self, id: u64) {
        self.subscribers.retain(|subscriber| subscriber.id != id);
    }

    pub(crate) fn pending_deliveries(
        &self,
        committed_batch: &[EventRecord],
    ) -> Vec<PendingDelivery> {
        if committed_batch.is_empty() {
            return Vec::new();
        }

        self.subscribers
            .iter()
            .filter_map(|subscriber| {
                let delivered_batch = match &subscriber.event_query {
                    None => committed_batch.to_vec(),
                    Some(event_query) => committed_batch
                        .iter()
                        .filter(|event_record| matches_query(event_query, event_record))
                        .cloned()
                        .collect(),
                };

                if delivered_batch.is_empty() {
                    return None;
                }

                // Delivery is snapshotted at commit time. Unsubscribe stops future deliveries,
                // but a batch already snapshotted here may still run later on the dispatcher.
                Some(PendingDelivery {
                    subscription_id: subscriber.id,
                    delivered_batch,
                    handle: subscriber.handle.clone(),
                })
            })
            .collect()
    }
}

impl PendingDelivery {
    pub(crate) fn deliver(self) {
        match catch_unwind(AssertUnwindSafe(|| (self.handle)(self.delivered_batch))) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                eprintln!(
                    "factstore-memory subscription handler {} failed after commit: {}",
                    self.subscription_id, error
                );
            }
            Err(_) => {
                eprintln!(
                    "factstore-memory subscription handler {} panicked after commit",
                    self.subscription_id
                );
            }
        }
    }
}
