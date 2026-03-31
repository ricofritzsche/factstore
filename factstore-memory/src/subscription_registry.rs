use std::sync::mpsc::{self, Sender};

use factstore::{EventRecord, LiveSubscription};

#[derive(Debug, Default)]
pub(crate) struct SubscriptionRegistry {
    subscribers: Vec<Sender<Vec<EventRecord>>>,
}

impl SubscriptionRegistry {
    pub(crate) fn subscribe(&mut self) -> LiveSubscription {
        let (sender, receiver) = mpsc::channel();
        self.subscribers.push(sender);
        LiveSubscription::new(receiver)
    }

    pub(crate) fn notify(&mut self, committed_batch: &[EventRecord]) {
        if committed_batch.is_empty() {
            return;
        }

        let committed_batch = committed_batch.to_vec();
        self.subscribers
            .retain(|subscriber| subscriber.send(committed_batch.clone()).is_ok());
    }
}
