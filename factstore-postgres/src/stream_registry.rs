use std::panic::{AssertUnwindSafe, catch_unwind};

use factstore::{EventQuery, EventRecord, HandleStream};

use crate::query_match::matches_query;

#[derive(Default)]
pub(crate) struct SubscriptionRegistry {
    next_subscription_id: u64,
    subscribers: Vec<Subscriber>,
}

struct Subscriber {
    id: u64,
    event_query: Option<EventQuery>,
    handle: HandleStream,
    durable_stream_id: Option<String>,
    live_delivery: LiveDelivery,
}

enum LiveDelivery {
    Live,
    Replaying {
        replay_until_sequence_number: u64,
        buffered_deliveries: Vec<PendingDelivery>,
    },
}

pub(crate) struct PendingDelivery {
    pub(crate) subscription_id: u64,
    pub(crate) durable_stream_id: Option<String>,
    pub(crate) last_processed_sequence_number: u64,
    pub(crate) delivered_batch: Vec<EventRecord>,
    pub(crate) handle: HandleStream,
}

pub(crate) enum DeliveryOutcome {
    Succeeded {
        subscription_id: u64,
        durable_stream_id: Option<String>,
        last_processed_sequence_number: u64,
    },
    Failed {
        subscription_id: u64,
        durable_stream_id: Option<String>,
    },
    Panicked {
        subscription_id: u64,
        durable_stream_id: Option<String>,
    },
}

impl SubscriptionRegistry {
    pub(crate) fn subscribe_all(&mut self, handle: HandleStream) -> u64 {
        self.subscribe_to(None, handle)
    }

    pub(crate) fn subscribe_to(
        &mut self,
        event_query: Option<EventQuery>,
        handle: HandleStream,
    ) -> u64 {
        self.insert_subscriber(event_query, handle, None, LiveDelivery::Live)
    }

    pub(crate) fn subscribe_all_durable(
        &mut self,
        durable_stream_id: String,
        replay_until_sequence_number: u64,
        handle: HandleStream,
    ) -> Result<u64, String> {
        self.subscribe_to_durable(
            durable_stream_id,
            None,
            replay_until_sequence_number,
            handle,
        )
    }

    pub(crate) fn subscribe_to_durable(
        &mut self,
        durable_stream_id: String,
        event_query: Option<EventQuery>,
        replay_until_sequence_number: u64,
        handle: HandleStream,
    ) -> Result<u64, String> {
        if self
            .subscribers
            .iter()
            .any(|subscriber| subscriber.durable_stream_id.as_deref() == Some(&durable_stream_id))
        {
            return Err(format!(
                "durable stream {durable_stream_id} is already active"
            ));
        }

        Ok(self.insert_subscriber(
            event_query,
            handle,
            Some(durable_stream_id),
            LiveDelivery::Replaying {
                replay_until_sequence_number,
                buffered_deliveries: Vec::new(),
            },
        ))
    }

    fn insert_subscriber(
        &mut self,
        event_query: Option<EventQuery>,
        handle: HandleStream,
        durable_stream_id: Option<String>,
        live_delivery: LiveDelivery,
    ) -> u64 {
        let id = self.next_subscription_id + 1;
        self.next_subscription_id = id;
        self.subscribers.push(Subscriber {
            id,
            event_query,
            handle,
            durable_stream_id,
            live_delivery,
        });
        id
    }

    pub(crate) fn unsubscribe(&mut self, id: u64) {
        self.subscribers.retain(|subscriber| subscriber.id != id);
    }

    pub(crate) fn pending_deliveries(
        &mut self,
        committed_batch: &[EventRecord],
    ) -> Vec<PendingDelivery> {
        if committed_batch.is_empty() {
            return Vec::new();
        }

        let last_processed_sequence_number = committed_batch
            .last()
            .expect("committed batch should not be empty")
            .sequence_number;
        let mut pending_deliveries = Vec::new();

        for subscriber in &mut self.subscribers {
            let durable_stream_id = subscriber.durable_stream_id.clone();
            let should_keep_empty_delivery = durable_stream_id.is_some();
            let delivered_batch = match &subscriber.event_query {
                None => committed_batch.to_vec(),
                Some(event_query) => committed_batch
                    .iter()
                    .filter(|event_record| matches_query(event_query, event_record))
                    .cloned()
                    .collect(),
            };

            if delivered_batch.is_empty() && !should_keep_empty_delivery {
                continue;
            }

            // Delivery is snapshotted at commit time. Unsubscribe stops future
            // deliveries, but a batch already snapshotted here may still run
            // later on the dispatcher.
            let pending_delivery = PendingDelivery {
                subscription_id: subscriber.id,
                durable_stream_id,
                last_processed_sequence_number,
                delivered_batch,
                handle: subscriber.handle.clone(),
            };

            match &mut subscriber.live_delivery {
                LiveDelivery::Live => pending_deliveries.push(pending_delivery),
                LiveDelivery::Replaying {
                    replay_until_sequence_number,
                    buffered_deliveries,
                } => {
                    if last_processed_sequence_number > *replay_until_sequence_number {
                        buffered_deliveries.push(pending_delivery);
                    }
                }
            }
        }

        pending_deliveries
    }

    pub(crate) fn finish_replay(&mut self, id: u64) -> Vec<PendingDelivery> {
        let Some(subscriber) = self
            .subscribers
            .iter_mut()
            .find(|subscriber| subscriber.id == id)
        else {
            return Vec::new();
        };

        match std::mem::replace(&mut subscriber.live_delivery, LiveDelivery::Live) {
            LiveDelivery::Live => Vec::new(),
            LiveDelivery::Replaying {
                buffered_deliveries,
                ..
            } => buffered_deliveries,
        }
    }
}

impl PendingDelivery {
    pub(crate) fn deliver(self) -> DeliveryOutcome {
        if self.delivered_batch.is_empty() {
            return DeliveryOutcome::Succeeded {
                subscription_id: self.subscription_id,
                durable_stream_id: self.durable_stream_id,
                last_processed_sequence_number: self.last_processed_sequence_number,
            };
        }

        match catch_unwind(AssertUnwindSafe(|| (self.handle)(self.delivered_batch))) {
            Ok(Ok(())) => DeliveryOutcome::Succeeded {
                subscription_id: self.subscription_id,
                durable_stream_id: self.durable_stream_id,
                last_processed_sequence_number: self.last_processed_sequence_number,
            },
            Ok(Err(error)) => {
                eprintln!(
                    "factstore-postgres stream handler {} failed after commit: {}",
                    self.subscription_id, error
                );

                DeliveryOutcome::Failed {
                    subscription_id: self.subscription_id,
                    durable_stream_id: self.durable_stream_id,
                }
            }
            Err(_) => {
                eprintln!(
                    "factstore-postgres stream handler {} panicked after commit",
                    self.subscription_id
                );

                DeliveryOutcome::Panicked {
                    subscription_id: self.subscription_id,
                    durable_stream_id: self.durable_stream_id,
                }
            }
        }
    }
}
