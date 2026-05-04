use napi::Error;

use factstr::EventStoreError;

pub(crate) fn napi_error_from_event_store_error(event_store_error: EventStoreError) -> Error {
    let interop_error = factstr_interop::InteropError::from(event_store_error);
    let code = match &interop_error {
        factstr_interop::InteropError::EmptyAppend => "EmptyAppend",
        factstr_interop::InteropError::ConditionalAppendConflict(_) => "ConditionalAppendConflict",
        factstr_interop::InteropError::NotImplemented { .. } => "NotImplemented",
        factstr_interop::InteropError::BackendFailure { .. } => "BackendFailure",
    };

    Error::from_reason(format!("{}: {}", code, interop_error))
}
