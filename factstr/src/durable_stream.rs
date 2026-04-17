/// Stable durable stream identity used to resume replay and catch-up from a
/// persisted cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableStream {
    name: String,
}

impl DurableStream {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
