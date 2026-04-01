use crate::{
    AppendResult, DurableStream, EventQuery, EventStream, HandleStream, NewEvent, QueryResult,
};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventStoreError {
    EmptyAppend,
    ConditionalAppendConflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    NotImplemented {
        store: &'static str,
        operation: &'static str,
    },
    BackendFailure {
        message: String,
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
            Self::NotImplemented { store, operation } => {
                write!(formatter, "{store} does not implement {operation} yet")
            }
            Self::BackendFailure { message } => write!(formatter, "backend failure: {message}"),
        }
    }
}

impl Error for EventStoreError {}

/// Shared event-store contract across all store implementations.
///
/// Load-bearing sequence guarantees:
/// - sequence numbers are global and monotonically increasing
/// - one committed batch receives one consecutive sequence range
/// - failed appends and failed conditional appends must not partially commit a batch
/// - under the current Rust contract, failed appends also must not consume
///   sequence numbers that later successful commits would observe
/// - streams observe only batches committed after stream registration becomes active
/// - `stream_all` delivers each successful committed append to the handler as
///   one committed batch in commit order
/// - `stream_to` uses `EventQuery` matching semantics to deliver only the
///   matching facts from each future committed batch, preserving their original
///   committed order inside one filtered delivered batch
/// - durable streams persist their last processed sequence number
/// - durable replay starts strictly after the stored durable cursor
/// - durable replay transitions into future committed stream delivery without
///   duplicates or gaps
/// - durable cursors must not advance past undelivered committed facts
/// - failed conditional appends deliver nothing
/// - handler failure must not weaken append correctness
pub trait EventStore {
    fn query(&self, event_query: &EventQuery) -> Result<QueryResult, EventStoreError>;

    fn append(&self, new_events: Vec<NewEvent>) -> Result<AppendResult, EventStoreError>;

    fn append_if(
        &self,
        new_events: Vec<NewEvent>,
        context_query: &EventQuery,
        expected_context_version: Option<u64>,
    ) -> Result<AppendResult, EventStoreError>;

    fn stream_all(&self, handle: HandleStream) -> Result<EventStream, EventStoreError>;

    fn stream_to(
        &self,
        event_query: &EventQuery,
        handle: HandleStream,
    ) -> Result<EventStream, EventStoreError>;

    fn stream_all_durable(
        &self,
        durable_stream: &DurableStream,
        handle: HandleStream,
    ) -> Result<EventStream, EventStoreError>;

    fn stream_to_durable(
        &self,
        durable_stream: &DurableStream,
        event_query: &EventQuery,
        handle: HandleStream,
    ) -> Result<EventStream, EventStoreError>;
}
