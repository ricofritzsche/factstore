use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventRecord {
    pub sequence_number: u64,
    pub event_type: String,
    pub payload: Value,
}
