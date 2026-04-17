use crate::EventRecord;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub type HandleStream =
    Arc<dyn Fn(Vec<EventRecord>) -> Result<(), StreamHandlerError> + Send + Sync + 'static>;

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
