use std::cell::{Cell, RefCell};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};
use std::thread::{self, JoinHandle};

use factstore::{
    AppendResult, EventQuery, EventRecord, EventStore, EventStoreError, EventSubscription,
    HandleEvents, NewEvent, QueryResult,
};

use crate::query_match::matches_query;
use crate::subscription_registry::{PendingDelivery, SubscriptionRegistry};

#[derive(Clone, Debug)]
struct CommittedAppend {
    append_result: AppendResult,
    event_records: Vec<EventRecord>,
}

enum DeliveryCommand {
    Deliver(Vec<PendingDelivery>),
    Shutdown,
}

pub struct MemoryStore {
    event_records: RefCell<Vec<EventRecord>>,
    next_sequence_number: Cell<u64>,
    subscription_registry: Arc<Mutex<SubscriptionRegistry>>,
    delivery_sender: Sender<DeliveryCommand>,
    delivery_thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    pub fn new() -> Self {
        let subscription_registry = Arc::new(Mutex::new(SubscriptionRegistry::default()));
        let (delivery_sender, delivery_receiver) = mpsc::channel();
        let delivery_thread = thread::Builder::new()
            .name("factstore-memory-delivery".to_owned())
            .spawn(move || run_delivery_thread(delivery_receiver))
            .expect("memory delivery thread should start");

        Self {
            event_records: RefCell::new(Vec::new()),
            next_sequence_number: Cell::new(1),
            subscription_registry,
            delivery_sender,
            delivery_thread: Mutex::new(Some(delivery_thread)),
        }
    }

    fn current_context_version(&self, event_query: &EventQuery) -> Option<u64> {
        self.event_records
            .borrow()
            .iter()
            .filter(|event_record| matches_query(event_query, event_record))
            .map(|event_record| event_record.sequence_number)
            .last()
    }

    fn append_records(
        &self,
        new_events: Vec<NewEvent>,
    ) -> Result<CommittedAppend, EventStoreError> {
        if new_events.is_empty() {
            return Err(EventStoreError::EmptyAppend);
        }

        let committed_count = new_events.len() as u64;
        let first_sequence_number = self.next_sequence_number.get();
        let last_sequence_number = first_sequence_number + committed_count - 1;
        let committed_event_records = new_events
            .into_iter()
            .enumerate()
            .map(|(offset, new_event)| EventRecord {
                sequence_number: first_sequence_number + offset as u64,
                event_type: new_event.event_type,
                payload: new_event.payload,
            })
            .collect::<Vec<_>>();

        self.event_records
            .borrow_mut()
            .extend(committed_event_records.iter().cloned());

        self.next_sequence_number.set(last_sequence_number + 1);

        Ok(CommittedAppend {
            append_result: AppendResult {
                first_sequence_number,
                last_sequence_number,
                committed_count,
            },
            event_records: committed_event_records,
        })
    }

    fn pending_deliveries(&self, committed_batch: &[EventRecord]) -> Vec<PendingDelivery> {
        match self.subscription_registry.lock() {
            Ok(subscription_registry) => subscription_registry.pending_deliveries(committed_batch),
            Err(poisoned) => poisoned.into_inner().pending_deliveries(committed_batch),
        }
    }

    fn enqueue_delivery(&self, pending_deliveries: Vec<PendingDelivery>) {
        if pending_deliveries.is_empty() {
            return;
        }

        if let Err(error) = self
            .delivery_sender
            .send(DeliveryCommand::Deliver(pending_deliveries))
        {
            eprintln!(
                "factstore-memory delivery dispatcher stopped after commit: {}",
                error
            );
        }
    }
}

impl Drop for MemoryStore {
    fn drop(&mut self) {
        let _ = self.delivery_sender.send(DeliveryCommand::Shutdown);

        if let Ok(mut delivery_thread) = self.delivery_thread.lock() {
            if let Some(delivery_thread) = delivery_thread.take() {
                let _ = delivery_thread.join();
            }
        }
    }
}

impl EventStore for MemoryStore {
    fn query(&self, event_query: &EventQuery) -> Result<QueryResult, EventStoreError> {
        let current_context_version = self.current_context_version(event_query);
        let event_records: Vec<EventRecord> = self
            .event_records
            .borrow()
            .iter()
            .filter(|event_record| matches_query(event_query, event_record))
            .filter(|event_record| {
                event_query
                    .min_sequence_number
                    .is_none_or(|min_sequence_number| {
                        event_record.sequence_number > min_sequence_number
                    })
            })
            .cloned()
            .collect();

        let last_returned_sequence_number = event_records
            .last()
            .map(|event_record| event_record.sequence_number);

        Ok(QueryResult {
            event_records,
            last_returned_sequence_number,
            current_context_version,
        })
    }

    fn append(&self, new_events: Vec<NewEvent>) -> Result<AppendResult, EventStoreError> {
        let committed_append = self.append_records(new_events)?;
        let pending_deliveries = self.pending_deliveries(&committed_append.event_records);
        self.enqueue_delivery(pending_deliveries);
        Ok(committed_append.append_result)
    }

    fn append_if(
        &self,
        new_events: Vec<NewEvent>,
        context_query: &EventQuery,
        expected_context_version: Option<u64>,
    ) -> Result<AppendResult, EventStoreError> {
        let actual_context_version = self.current_context_version(context_query);

        if actual_context_version != expected_context_version {
            return Err(EventStoreError::ConditionalAppendConflict {
                expected: expected_context_version,
                actual: actual_context_version,
            });
        }

        let committed_append = self.append_records(new_events)?;
        let pending_deliveries = self.pending_deliveries(&committed_append.event_records);
        self.enqueue_delivery(pending_deliveries);
        Ok(committed_append.append_result)
    }

    fn subscribe_all(&self, handle: HandleEvents) -> Result<EventSubscription, EventStoreError> {
        let subscription_registry = Arc::clone(&self.subscription_registry);
        let id = match subscription_registry.lock() {
            Ok(mut subscription_registry) => subscription_registry.subscribe_all(handle),
            Err(poisoned) => poisoned.into_inner().subscribe_all(handle),
        };

        Ok(EventSubscription::new(
            id,
            Arc::new(move |subscription_id| match subscription_registry.lock() {
                Ok(mut subscription_registry) => subscription_registry.unsubscribe(subscription_id),
                Err(poisoned) => poisoned.into_inner().unsubscribe(subscription_id),
            }),
        ))
    }

    fn subscribe_to(
        &self,
        event_query: &EventQuery,
        handle: HandleEvents,
    ) -> Result<EventSubscription, EventStoreError> {
        let subscription_registry = Arc::clone(&self.subscription_registry);
        let id = match subscription_registry.lock() {
            Ok(mut subscription_registry) => {
                subscription_registry.subscribe_to(Some(event_query.clone()), handle)
            }
            Err(poisoned) => poisoned
                .into_inner()
                .subscribe_to(Some(event_query.clone()), handle),
        };

        Ok(EventSubscription::new(
            id,
            Arc::new(move |subscription_id| match subscription_registry.lock() {
                Ok(mut subscription_registry) => subscription_registry.unsubscribe(subscription_id),
                Err(poisoned) => poisoned.into_inner().unsubscribe(subscription_id),
            }),
        ))
    }
}

fn run_delivery_thread(delivery_receiver: Receiver<DeliveryCommand>) {
    while let Ok(delivery_command) = delivery_receiver.recv() {
        match delivery_command {
            DeliveryCommand::Deliver(pending_deliveries) => {
                for pending_delivery in pending_deliveries {
                    pending_delivery.deliver();
                }
            }
            DeliveryCommand::Shutdown => break,
        }
    }
}
