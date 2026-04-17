use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventFilter {
    /// `None` means event type is unconstrained.
    /// `Some(Vec::new())` means this filter matches no event type.
    pub event_types: Option<Vec<String>>,
    /// `None` means payload is unconstrained.
    /// `Some(Vec::new())` means this filter matches no payload predicate.
    pub payload_predicates: Option<Vec<Value>>,
}

impl EventFilter {
    pub fn for_event_types(event_types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            event_types: Some(event_types.into_iter().map(Into::into).collect()),
            payload_predicates: None,
        }
    }

    pub fn with_payload_predicates(
        mut self,
        payload_predicates: impl IntoIterator<Item = Value>,
    ) -> Self {
        self.payload_predicates = Some(payload_predicates.into_iter().collect());
        self
    }
}
