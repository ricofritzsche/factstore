use std::ptr;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::thread::{self, ThreadId};

use factstr::{EventRecord as FactstrEventRecord, HandleStream, StreamHandlerError};
use napi::bindgen_prelude::{FromNapiValue, ToNapiValue};
use napi::threadsafe_function::{
    ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi::{
    CallContext, Env, JsFunction, JsObject, JsUnknown, NapiRaw, NapiValue, Result, Status,
    ValueType, sys,
};

use crate::EventRecord;

struct JsFunctionReference {
    env: sys::napi_env,
    function_ref: sys::napi_ref,
}

unsafe impl Send for JsFunctionReference {}
unsafe impl Sync for JsFunctionReference {}

impl JsFunctionReference {
    fn new(env: sys::napi_env, function: sys::napi_value) -> Result<Self> {
        Ok(Self {
            env,
            function_ref: create_js_reference(env, function)?,
        })
    }

    fn load(&self) -> Result<JsFunction> {
        load_referenced_function(self.env, self.function_ref)
    }
}

impl Drop for JsFunctionReference {
    fn drop(&mut self) {
        let _ = unsafe { sys::napi_delete_reference(self.env, self.function_ref) };
    }
}

struct JsStreamCallback {
    env: sys::napi_env,
    trampoline_ref: sys::napi_ref,
    registration_thread_id: ThreadId,
    threadsafe_handle: ThreadsafeFunction<Vec<EventRecord>, ErrorStrategy::CalleeHandled>,
}

unsafe impl Send for JsStreamCallback {}
unsafe impl Sync for JsStreamCallback {}

impl JsStreamCallback {
    fn new(env: Env, handle: JsFunction) -> Result<Self> {
        let raw_function = unsafe { handle.raw() };
        let callback_ref = Arc::new(JsFunctionReference::new(env.raw(), raw_function)?);
        let trampoline = create_stream_callback_trampoline(env, callback_ref.clone())?;
        let raw_trampoline = unsafe { trampoline.raw() };
        let threadsafe_handle: ThreadsafeFunction<Vec<EventRecord>, ErrorStrategy::CalleeHandled> =
            trampoline.create_threadsafe_function(
                0,
                |context: ThreadSafeCallContext<Vec<EventRecord>>| {
                    let js_event_records = event_records_to_js_array(&context.env, context.value)?;
                    Ok(vec![js_event_records])
                },
            )?;

        Ok(Self {
            env: env.raw(),
            trampoline_ref: create_js_reference(env.raw(), raw_trampoline)?,
            registration_thread_id: thread::current().id(),
            threadsafe_handle,
        })
    }

    fn invoke_direct(&self, node_event_records: Vec<EventRecord>) -> Result<bool> {
        let env = unsafe { Env::from_raw(self.env) };
        let function = load_referenced_function(self.env, self.trampoline_ref)?;
        let js_event_records = event_records_to_js_array(&env, node_event_records)?;
        let return_value = function.call(None, &[js_event_records])?;

        callback_completed_successfully(self.env, return_value)
    }

    fn invoke_via_threadsafe_function(&self, node_event_records: Vec<EventRecord>) -> Result<bool> {
        let (sender, receiver) = sync_channel(1);
        let status = self.threadsafe_handle.call_with_return_value(
            Ok(node_event_records),
            ThreadsafeFunctionCallMode::Blocking,
            move |callback_succeeded| {
                sender
                    .send(callback_succeeded)
                    .map_err(|error| napi::Error::from_reason(error.to_string()))
            },
        );

        if status != Status::Ok {
            return Err(napi::Error::from_status(status));
        }

        receiver
            .recv()
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    fn dispatch(
        &self,
        event_records: Vec<FactstrEventRecord>,
    ) -> StdResult<(), StreamHandlerError> {
        let node_event_records = event_records
            .into_iter()
            .map(|event_record| factstr_interop::InteropEventRecord::from(event_record).into())
            .collect::<Vec<_>>();

        let callback_succeeded = if thread::current().id() == self.registration_thread_id {
            self.invoke_direct(node_event_records)
        } else {
            self.invoke_via_threadsafe_function(node_event_records)
        }
        .map_err(|error| StreamHandlerError::new(error.to_string()))?;

        if callback_succeeded {
            Ok(())
        } else {
            Err(StreamHandlerError::new(
                "Node stream callback did not complete successfully",
            ))
        }
    }
}

impl Drop for JsStreamCallback {
    fn drop(&mut self) {
        let _ = unsafe { sys::napi_delete_reference(self.env, self.trampoline_ref) };
    }
}

pub(crate) fn handle_stream_from_js_function(env: Env, handle: JsFunction) -> Result<HandleStream> {
    let stream_callback = Arc::new(JsStreamCallback::new(env, handle)?);

    Ok(Arc::new(move |event_records: Vec<FactstrEventRecord>| {
        stream_callback.dispatch(event_records)
    }))
}

fn event_records_to_js_array(env: &Env, event_records: Vec<EventRecord>) -> Result<JsObject> {
    let mut js_event_records = env.create_array_with_length(event_records.len())?;

    for (index, event_record) in event_records.into_iter().enumerate() {
        let js_event_record =
            unsafe { <EventRecord as ToNapiValue>::to_napi_value(env.raw(), event_record) }?;
        let js_event_record = unsafe { napi::JsUnknown::from_raw(env.raw(), js_event_record) }?;
        js_event_records.set_element(index as u32, js_event_record)?;
    }

    Ok(js_event_records)
}

fn create_stream_callback_trampoline(
    env: Env,
    callback_ref: Arc<JsFunctionReference>,
) -> Result<JsFunction> {
    env.create_function_from_closure("factstrStreamCallback", move |ctx: CallContext| {
        let callback_succeeded =
            invoke_user_stream_callback(&ctx, callback_ref.as_ref()).unwrap_or(false);
        ctx.env.get_boolean(callback_succeeded)
    })
}

fn invoke_user_stream_callback(
    ctx: &CallContext<'_>,
    callback_ref: &JsFunctionReference,
) -> Result<bool> {
    let callback = callback_ref.load()?;
    let js_event_records = match stream_callback_events_argument(ctx)? {
        Some(js_event_records) => js_event_records,
        None => return Ok(false),
    };

    let return_value = match callback.call(None, &[js_event_records]) {
        Ok(return_value) => return_value,
        Err(_) => return Ok(false),
    };

    callback_completed_successfully(ctx.env.raw(), return_value)
}

fn stream_callback_events_argument(ctx: &CallContext<'_>) -> Result<Option<JsObject>> {
    if ctx.length == 0 {
        return Ok(None);
    }

    if ctx.length == 1 {
        return ctx.get::<JsObject>(0).map(Some);
    }

    let error_value = ctx.get::<JsUnknown>(0)?;
    match error_value.get_type()? {
        ValueType::Null | ValueType::Undefined => ctx.get::<JsObject>(1).map(Some),
        _ => Ok(None),
    }
}

fn callback_completed_successfully(env: sys::napi_env, return_value: JsUnknown) -> Result<bool> {
    match return_value.get_type()? {
        ValueType::Boolean => unsafe { bool::from_napi_value(env, return_value.raw()) },
        _ => Ok(true),
    }
}

fn create_js_reference(env: sys::napi_env, value: sys::napi_value) -> Result<sys::napi_ref> {
    let mut function_ref = ptr::null_mut();
    let status = unsafe { sys::napi_create_reference(env, value, 1, &mut function_ref) };
    if status != sys::Status::napi_ok {
        return Err(napi::Error::from_status(Status::from(status)));
    }

    Ok(function_ref)
}

fn load_referenced_function(env: sys::napi_env, function_ref: sys::napi_ref) -> Result<JsFunction> {
    let mut function_value = ptr::null_mut();
    let status = unsafe { sys::napi_get_reference_value(env, function_ref, &mut function_value) };
    if status != sys::Status::napi_ok {
        return Err(napi::Error::from_status(Status::from(status)));
    }

    unsafe { JsFunction::from_raw(env, function_value) }
}
