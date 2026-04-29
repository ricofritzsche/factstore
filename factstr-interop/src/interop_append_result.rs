use factstr::AppendResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InteropAppendResult {
    pub first_sequence_number: u64,
    pub last_sequence_number: u64,
    pub committed_count: u64,
}

impl From<InteropAppendResult> for AppendResult {
    fn from(interop_append_result: InteropAppendResult) -> Self {
        Self {
            first_sequence_number: interop_append_result.first_sequence_number,
            last_sequence_number: interop_append_result.last_sequence_number,
            committed_count: interop_append_result.committed_count,
        }
    }
}

impl From<AppendResult> for InteropAppendResult {
    fn from(append_result: AppendResult) -> Self {
        Self {
            first_sequence_number: append_result.first_sequence_number,
            last_sequence_number: append_result.last_sequence_number,
            committed_count: append_result.committed_count,
        }
    }
}
