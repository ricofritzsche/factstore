#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendResult {
    pub first_sequence_number: u64,
    pub last_sequence_number: u64,
    pub committed_count: u64,
}
