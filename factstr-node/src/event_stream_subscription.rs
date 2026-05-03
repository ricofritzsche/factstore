use factstr::EventStream;
use napi_derive::napi;

#[napi]
pub struct EventStreamSubscription {
    event_stream: EventStream,
}

impl EventStreamSubscription {
    pub(crate) fn new(event_stream: EventStream) -> Self {
        Self { event_stream }
    }
}

#[napi]
impl EventStreamSubscription {
    #[napi]
    pub fn unsubscribe(&self) {
        self.event_stream.unsubscribe();
    }
}
