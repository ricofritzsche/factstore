use crate::EventRecord;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub type HandleEvents =
    Arc<dyn Fn(Vec<EventRecord>) -> Result<(), SubscriptionHandlerError> + Send + Sync + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionHandlerError {
    message: String,
}

impl SubscriptionHandlerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for SubscriptionHandlerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "subscription handler failed: {}", self.message)
    }
}

impl Error for SubscriptionHandlerError {}
