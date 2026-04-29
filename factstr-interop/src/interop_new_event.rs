use factstr::NewEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InteropNewEvent {
    pub event_type: String,
    pub payload: Value,
}

impl InteropNewEvent {
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
        }
    }
}

impl From<InteropNewEvent> for NewEvent {
    fn from(interop_new_event: InteropNewEvent) -> Self {
        Self {
            event_type: interop_new_event.event_type,
            payload: interop_new_event.payload,
        }
    }
}

impl From<NewEvent> for InteropNewEvent {
    fn from(new_event: NewEvent) -> Self {
        Self {
            event_type: new_event.event_type,
            payload: new_event.payload,
        }
    }
}
