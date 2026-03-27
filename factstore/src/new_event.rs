use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewEvent {
    pub event_type: String,
    pub payload: Value,
}

impl NewEvent {
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
        }
    }
}
