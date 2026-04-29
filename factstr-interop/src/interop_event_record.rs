use factstr::EventRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InteropEventRecord {
    pub sequence_number: u64,
    pub event_type: String,
    pub payload: Value,
}

impl From<InteropEventRecord> for EventRecord {
    fn from(interop_event_record: InteropEventRecord) -> Self {
        Self {
            sequence_number: interop_event_record.sequence_number,
            event_type: interop_event_record.event_type,
            payload: interop_event_record.payload,
        }
    }
}

impl From<EventRecord> for InteropEventRecord {
    fn from(event_record: EventRecord) -> Self {
        Self {
            sequence_number: event_record.sequence_number,
            event_type: event_record.event_type,
            payload: event_record.payload,
        }
    }
}
