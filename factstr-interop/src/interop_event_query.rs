use crate::InteropEventFilter;
use factstr::EventQuery;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InteropEventQuery {
    pub filters: Option<Vec<InteropEventFilter>>,
    /// Exclusive read cursor for returned rows.
    ///
    /// This affects returned rows only. It does not narrow the full conflict
    /// context version reported by `query`, and it does not narrow the context
    /// checked by `append_if`.
    pub min_sequence_number: Option<u64>,
}

impl InteropEventQuery {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn with_filters(mut self, filters: impl IntoIterator<Item = InteropEventFilter>) -> Self {
        self.filters = Some(filters.into_iter().collect());
        self
    }

    pub fn for_event_types(event_types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            filters: Some(vec![InteropEventFilter::for_event_types(event_types)]),
            min_sequence_number: None,
        }
    }

    pub fn with_min_sequence_number(mut self, min_sequence_number: u64) -> Self {
        self.min_sequence_number = Some(min_sequence_number);
        self
    }
}

impl From<InteropEventQuery> for EventQuery {
    fn from(interop_event_query: InteropEventQuery) -> Self {
        Self {
            filters: interop_event_query
                .filters
                .map(|filters| filters.into_iter().map(Into::into).collect()),
            min_sequence_number: interop_event_query.min_sequence_number,
        }
    }
}

impl From<EventQuery> for InteropEventQuery {
    fn from(event_query: EventQuery) -> Self {
        Self {
            filters: event_query
                .filters
                .map(|filters| filters.into_iter().map(Into::into).collect()),
            min_sequence_number: event_query.min_sequence_number,
        }
    }
}
