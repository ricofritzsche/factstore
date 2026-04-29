use crate::InteropEventRecord;
use factstr::QueryResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InteropQueryResult {
    pub event_records: Vec<InteropEventRecord>,
    pub last_returned_sequence_number: Option<u64>,
    pub current_context_version: Option<u64>,
}

impl From<InteropQueryResult> for QueryResult {
    fn from(interop_query_result: InteropQueryResult) -> Self {
        Self {
            event_records: interop_query_result
                .event_records
                .into_iter()
                .map(Into::into)
                .collect(),
            last_returned_sequence_number: interop_query_result.last_returned_sequence_number,
            current_context_version: interop_query_result.current_context_version,
        }
    }
}

impl From<QueryResult> for InteropQueryResult {
    fn from(query_result: QueryResult) -> Self {
        Self {
            event_records: query_result
                .event_records
                .into_iter()
                .map(Into::into)
                .collect(),
            last_returned_sequence_number: query_result.last_returned_sequence_number,
            current_context_version: query_result.current_context_version,
        }
    }
}
