use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Active live subscription returned by `subscribe_all` or `subscribe_to`.
///
/// The id is stable for the lifetime of the subscription. Calling
/// `unsubscribe()` stops future delivery for that subscriber. Dropping the
/// handle also unsubscribes if it is still active.
pub struct EventSubscription {
    id: u64,
    unsubscribe: Arc<dyn Fn(u64) + Send + Sync + 'static>,
    unsubscribed: AtomicBool,
}

impl EventSubscription {
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

impl Drop for EventSubscription {
    fn drop(&mut self) {
        if !self.unsubscribed.swap(true, Ordering::SeqCst) {
            (self.unsubscribe)(self.id);
        }
    }
}
