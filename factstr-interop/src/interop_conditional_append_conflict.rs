use factstr::EventStoreError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InteropConditionalAppendConflict {
    pub expected_context_version: Option<u64>,
    pub actual_context_version: Option<u64>,
}

impl InteropConditionalAppendConflict {
    pub fn new(expected_context_version: Option<u64>, actual_context_version: Option<u64>) -> Self {
        Self {
            expected_context_version,
            actual_context_version,
        }
    }
}

impl From<InteropConditionalAppendConflict> for EventStoreError {
    fn from(conflict: InteropConditionalAppendConflict) -> Self {
        Self::ConditionalAppendConflict {
            expected: conflict.expected_context_version,
            actual: conflict.actual_context_version,
        }
    }
}
