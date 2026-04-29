use crate::InteropConditionalAppendConflict;
use factstr::EventStoreError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InteropError {
    EmptyAppend,
    ConditionalAppendConflict(InteropConditionalAppendConflict),
    NotImplemented { store: String, operation: String },
    BackendFailure { message: String },
}

impl Display for InteropError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAppend => formatter.write_str("append requires at least one event"),
            Self::ConditionalAppendConflict(conflict) => {
                write!(
                    formatter,
                    "context version mismatch: expected {:?}, actual {:?}",
                    conflict.expected_context_version, conflict.actual_context_version
                )
            }
            Self::NotImplemented { store, operation } => {
                write!(formatter, "{store} does not implement {operation} yet")
            }
            Self::BackendFailure { message } => write!(formatter, "backend failure: {message}"),
        }
    }
}

impl Error for InteropError {}

impl From<EventStoreError> for InteropError {
    fn from(event_store_error: EventStoreError) -> Self {
        match event_store_error {
            EventStoreError::EmptyAppend => Self::EmptyAppend,
            EventStoreError::ConditionalAppendConflict { expected, actual } => {
                Self::ConditionalAppendConflict(InteropConditionalAppendConflict {
                    expected_context_version: expected,
                    actual_context_version: actual,
                })
            }
            EventStoreError::NotImplemented { store, operation } => Self::NotImplemented {
                store: store.to_owned(),
                operation: operation.to_owned(),
            },
            EventStoreError::BackendFailure { message } => Self::BackendFailure { message },
        }
    }
}
