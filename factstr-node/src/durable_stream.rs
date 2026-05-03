use napi_derive::napi;

#[napi(object)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableStream {
    pub name: String,
}

impl DurableStream {
    pub fn into_factstr(self) -> factstr::DurableStream {
        factstr::DurableStream::new(self.name)
    }
}
