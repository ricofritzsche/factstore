use crate::EventFilter;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventQuery {
    pub filters: Option<Vec<EventFilter>>,
    /// Exclusive read cursor for returned rows.
    ///
    /// This affects returned rows only. It does not narrow the full conflict
    /// context version reported by `query`, and it does not narrow the context
    /// checked by `append_if`.
    pub min_sequence_number: Option<u64>,
}

impl EventQuery {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn with_filters(mut self, filters: impl IntoIterator<Item = EventFilter>) -> Self {
        self.filters = Some(filters.into_iter().collect());
        self
    }

    pub fn for_event_types(event_types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            filters: Some(vec![EventFilter::for_event_types(event_types)]),
            min_sequence_number: None,
        }
    }

    pub fn with_min_sequence_number(mut self, min_sequence_number: u64) -> Self {
        self.min_sequence_number = Some(min_sequence_number);
        self
    }
}
