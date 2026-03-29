/// Result of one committed append batch.
///
/// A successful append always commits one consecutive global sequence range.
/// Failed appends and failed conditional appends must not partially commit any
/// part of the batch, and under the current Rust contract they also must not
/// consume sequence numbers that later successful appends would observe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendResult {
    pub first_sequence_number: u64,
    pub last_sequence_number: u64,
    pub committed_count: u64,
}
