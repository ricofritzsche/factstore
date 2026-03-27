use crate::EventRecord;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryResult {
    pub event_records: Vec<EventRecord>,
    pub last_returned_sequence_number: Option<u64>,
    pub current_context_version: Option<u64>,
}
