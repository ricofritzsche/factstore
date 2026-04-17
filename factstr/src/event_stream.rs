use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Active stream registration returned by `stream_all`, `stream_to`,
/// `stream_all_durable`, or `stream_to_durable`.
///
/// The id is stable for the lifetime of the stream. Calling `unsubscribe()`
/// stops future delivery for that stream. Dropping the
/// handle also unsubscribes if it is still active.
pub struct EventStream {
    id: u64,
    unsubscribe: Arc<dyn Fn(u64) + Send + Sync + 'static>,
    unsubscribed: AtomicBool,
}

impl EventStream {
    #[doc(hidden)]
    pub fn new(id: u64, unsubscribe: Arc<dyn Fn(u64) + Send + Sync + 'static>) -> Self {
        Self {
            id,
            unsubscribe,
            unsubscribed: AtomicBool::new(false),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn unsubscribe(&self) {
        if !self.unsubscribed.swap(true, Ordering::SeqCst) {
            (self.unsubscribe)(self.id);
        }
    }
}

impl Drop for EventStream {
    fn drop(&mut self) {
        if !self.unsubscribed.swap(true, Ordering::SeqCst) {
            (self.unsubscribe)(self.id);
        }
    }
}
