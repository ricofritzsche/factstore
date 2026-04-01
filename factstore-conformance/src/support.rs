use factstore::{EventRecord, HandleStream, NewEvent, StreamHandlerError};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) fn new_event(event_type: &str, payload: Value) -> NewEvent {
    NewEvent::new(event_type, payload)
}

pub(crate) fn recording_handle(delivery_log: Arc<Mutex<Vec<Vec<EventRecord>>>>) -> HandleStream {
    Arc::new(move |event_records| {
        delivery_log
            .lock()
            .expect("delivery log lock should succeed")
            .push(event_records);
        Ok(())
    })
}

pub(crate) fn failing_handle() -> HandleStream {
    Arc::new(|_| Err(StreamHandlerError::new("expected test handler failure")))
}

pub(crate) fn delivered_batches(
    delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>,
) -> Vec<Vec<EventRecord>> {
    delivery_log
        .lock()
        .expect("delivery log lock should succeed")
        .clone()
}

pub(crate) fn wait_for_delivery_count(
    delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>,
    expected_count: usize,
) -> Vec<Vec<EventRecord>> {
    let deadline = Instant::now() + Duration::from_secs(1);

    loop {
        let delivered_batches = delivered_batches(delivery_log);
        if delivered_batches.len() == expected_count {
            return delivered_batches;
        }

        assert!(
            Instant::now() < deadline,
            "expected {expected_count} delivered batches, got {}",
            delivered_batches.len()
        );

        thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn assert_no_delivery(delivery_log: &Arc<Mutex<Vec<Vec<EventRecord>>>>) {
    let deadline = Instant::now() + Duration::from_millis(100);

    while Instant::now() < deadline {
        let delivered_batches = delivered_batches(delivery_log);
        assert!(
            delivered_batches.is_empty(),
            "expected no delivered batches, got {}",
            delivered_batches.len()
        );
        thread::sleep(Duration::from_millis(5));
    }
}
