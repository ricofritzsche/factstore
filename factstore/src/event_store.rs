use crate::{AppendResult, EventQuery, NewEvent, QueryResult};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventStoreError {
    EmptyAppend,
    ConditionalAppendConflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
}

impl Display for EventStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAppend => formatter.write_str("append requires at least one event"),
            Self::ConditionalAppendConflict { expected, actual } => {
                write!(
                    formatter,
                    "context version mismatch: expected {expected:?}, actual {actual:?}"
                )
            }
        }
    }
}

impl Error for EventStoreError {}

pub trait EventStore {
    fn query(&self, event_query: &EventQuery) -> Result<QueryResult, EventStoreError>;

    fn append(&self, new_events: Vec<NewEvent>) -> Result<AppendResult, EventStoreError>;

    fn append_if(
        &self,
        new_events: Vec<NewEvent>,
        context_query: &EventQuery,
        expected_context_version: Option<u64>,
    ) -> Result<AppendResult, EventStoreError>;
}
