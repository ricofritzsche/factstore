use crate::{InteropAppendResult, InteropConditionalAppendConflict};
use napi_derive::napi;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct AppendIfResult {
    #[napi(js_name = "append_result")]
    pub append_result: Option<InteropAppendResult>,
    pub conflict: Option<InteropConditionalAppendConflict>,
}
