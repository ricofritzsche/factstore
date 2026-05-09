use crate::EventRecord;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type StreamHandlerFuture =
    Pin<Box<dyn Future<Output = Result<(), StreamHandlerError>> + Send + 'static>>;

#[derive(Clone)]
pub struct HandleStream {
    inner: Arc<dyn Fn(Vec<EventRecord>) -> StreamHandlerFuture + Send + Sync + 'static>,
}

impl HandleStream {
    pub fn new<F, Fut>(handle: F) -> Self
    where
        F: Fn(Vec<EventRecord>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), StreamHandlerError>> + Send + 'static,
    {
        Self {
            inner: Arc::new(move |event_records| Box::pin(handle(event_records))),
        }
    }

    pub fn call(&self, event_records: Vec<EventRecord>) -> StreamHandlerFuture {
        (self.inner)(event_records)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamHandlerError {
    message: String,
}

impl StreamHandlerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for StreamHandlerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "stream handler failed: {}", self.message)
    }
}

impl Error for StreamHandlerError {}
