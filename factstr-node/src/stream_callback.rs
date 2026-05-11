use std::ptr;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{SyncSender, sync_channel};

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
    threadsafe_handle: ThreadsafeFunction<PendingStreamCallbackInvocation, ErrorStrategy::Fatal>,
}

unsafe impl Send for JsStreamCallback {}
unsafe impl Sync for JsStreamCallback {}

struct PendingStreamCallbackInvocation {
    event_records: Vec<EventRecord>,
    completion_sender: Arc<Mutex<Option<SyncSender<bool>>>>,
}

impl JsStreamCallback {
    fn new(env: Env, handle: JsFunction) -> Result<Self> {
        let raw_function = unsafe { handle.raw() };
        let callback_ref = Arc::new(JsFunctionReference::new(env.raw(), raw_function)?);
        let trampoline = create_stream_callback_trampoline(env, callback_ref.clone())?;
        let raw_trampoline = unsafe { trampoline.raw() };
        let threadsafe_handle: ThreadsafeFunction<
            PendingStreamCallbackInvocation,
            ErrorStrategy::Fatal,
        > = trampoline.create_threadsafe_function(
            0,
            |context: ThreadSafeCallContext<PendingStreamCallbackInvocation>| {
                let js_event_records =
                    event_records_to_js_array(&context.env, context.value.event_records)?;
                let completion_callback = create_stream_completion_callback(
                    context.env,
                    context.value.completion_sender,
                )?;
                let js_event_records = js_event_records.into_unknown();
                let completion_callback =
                    unsafe { JsUnknown::from_raw(context.env.raw(), completion_callback.raw()) }?;
                Ok(vec![js_event_records, completion_callback])
            },
        )?;

        Ok(Self {
            env: env.raw(),
            trampoline_ref: create_js_reference(env.raw(), raw_trampoline)?,
            threadsafe_handle,
        })
    }

    fn invoke_via_threadsafe_function(&self, node_event_records: Vec<EventRecord>) -> Result<bool> {
        let (setup_sender, setup_receiver) = sync_channel(1);
        let (completion_sender, completion_receiver) = sync_channel(1);
        let invocation = PendingStreamCallbackInvocation {
            event_records: node_event_records,
            completion_sender: Arc::new(Mutex::new(Some(completion_sender))),
        };
        let status = self
            .threadsafe_handle
            .call_with_return_value::<JsUnknown, _>(
                invocation,
                ThreadsafeFunctionCallMode::Blocking,
                move |_return_value| {
                    setup_sender
                        .send(())
                        .map_err(|error| napi::Error::from_reason(error.to_string()))
                        .map(|_| ())
                },
            );

        if status != Status::Ok {
            return Err(napi::Error::from_status(status));
        }

        setup_receiver
            .recv()
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;

        completion_receiver
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

        let callback_succeeded = self
            .invoke_via_threadsafe_function(node_event_records)
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

    Ok(HandleStream::new(
        move |event_records: Vec<FactstrEventRecord>| {
            let stream_callback = Arc::clone(&stream_callback);

            async move { stream_callback.dispatch(event_records) }
        },
    ))
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
        let _ = invoke_user_stream_callback(&ctx, callback_ref.as_ref());
        ctx.env.get_undefined()
    })
}

fn invoke_user_stream_callback(
    ctx: &CallContext<'_>,
    callback_ref: &JsFunctionReference,
) -> Result<()> {
    let complete = match stream_callback_completion_argument(ctx) {
        Ok(complete) => complete,
        Err(_) => return Ok(()),
    };
    let complete_ref =
        match JsFunctionReference::new(ctx.env.raw(), unsafe { complete.raw() }).map(Arc::new) {
            Ok(complete_ref) => complete_ref,
            Err(_) => {
                let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
                return Ok(());
            }
        };
    let callback = match callback_ref.load() {
        Ok(callback) => callback,
        Err(_) => {
            let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
            return Ok(());
        }
    };
    let js_event_records = match stream_callback_events_argument(ctx) {
        Ok(Some(js_event_records)) => js_event_records,
        Ok(None) | Err(_) => {
            let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
            return Ok(());
        }
    };

    let return_value = match callback.call(None, &[js_event_records]) {
        Ok(return_value) => return_value,
        Err(_) => {
            let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
            return Ok(());
        }
    };

    match thenable_from_return_value(*ctx.env, &return_value) {
        Ok(Some((thenable, then_function))) => {
            let on_fulfilled =
                match create_thenable_resolution_callback(*ctx.env, Arc::clone(&complete_ref)) {
                    Ok(on_fulfilled) => on_fulfilled,
                    Err(_) => {
                        let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
                        return Ok(());
                    }
                };
            let on_rejected = match create_thenable_rejection_callback(*ctx.env, complete_ref) {
                Ok(on_rejected) => on_rejected,
                Err(_) => {
                    let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
                    return Ok(());
                }
            };
            if then_function
                .call(Some(&thenable), &[on_fulfilled, on_rejected])
                .is_err()
            {
                let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
            }
        }
        Ok(None) => {
            let callback_succeeded =
                callback_completed_successfully(ctx.env.raw(), return_value).unwrap_or(false);
            let _ = complete_stream_callback(ctx.env.raw(), &complete, callback_succeeded);
        }
        Err(_) => {
            let _ = complete_stream_callback(ctx.env.raw(), &complete, false);
        }
    }

    Ok(())
}

fn stream_callback_events_argument(ctx: &CallContext<'_>) -> Result<Option<JsObject>> {
    if ctx.length < 2 {
        return Ok(None);
    }

    ctx.get::<JsObject>(0).map(Some)
}

fn stream_callback_completion_argument(ctx: &CallContext<'_>) -> Result<JsFunction> {
    ctx.get::<JsFunction>(1)
}

fn callback_completed_successfully(env: sys::napi_env, return_value: JsUnknown) -> Result<bool> {
    match return_value.get_type()? {
        ValueType::Boolean => unsafe { bool::from_napi_value(env, return_value.raw()) },
        _ => Ok(true),
    }
}

fn complete_stream_callback(
    env: sys::napi_env,
    complete: &JsFunction,
    callback_succeeded: bool,
) -> Result<()> {
    let env = unsafe { Env::from_raw(env) };
    let js_callback_succeeded = env.get_boolean(callback_succeeded)?;
    complete.call(None, &[js_callback_succeeded])?;
    Ok(())
}

fn complete_stream_callback_with_reference(
    env: sys::napi_env,
    complete_ref: &JsFunctionReference,
    callback_succeeded: bool,
) -> Result<()> {
    let complete = complete_ref.load()?;
    let env = unsafe { Env::from_raw(env) };
    let js_callback_succeeded = env.get_boolean(callback_succeeded)?;
    complete.call(None, &[js_callback_succeeded])?;
    Ok(())
}

fn thenable_from_return_value(
    env: Env,
    return_value: &JsUnknown,
) -> Result<Option<(JsObject, JsFunction)>> {
    match return_value.get_type()? {
        ValueType::Object | ValueType::Function => {
            let thenable = unsafe { JsUnknown::from_raw(env.raw(), return_value.raw()) }?
                .coerce_to_object()?;
            if !thenable.has_named_property("then")? {
                return Ok(None);
            }

            let then_value = thenable.get_named_property::<JsUnknown>("then")?;
            if then_value.get_type()? != ValueType::Function {
                return Ok(None);
            }

            let then_function = unsafe { JsFunction::from_raw(env.raw(), then_value.raw()) }?;
            Ok(Some((thenable, then_function)))
        }
        _ => Ok(None),
    }
}

fn create_thenable_resolution_callback(
    env: Env,
    complete_ref: Arc<JsFunctionReference>,
) -> Result<JsFunction> {
    env.create_function_from_closure("factstrStreamCallbackResolved", move |ctx: CallContext| {
        let callback_succeeded = if ctx.length == 0 {
            true
        } else {
            match ctx.get::<JsUnknown>(0) {
                Ok(return_value) => {
                    callback_completed_successfully(ctx.env.raw(), return_value).unwrap_or(false)
                }
                Err(_) => false,
            }
        };
        let _ = complete_stream_callback_with_reference(
            ctx.env.raw(),
            complete_ref.as_ref(),
            callback_succeeded,
        );
        ctx.env.get_undefined()
    })
}

fn create_thenable_rejection_callback(
    env: Env,
    complete_ref: Arc<JsFunctionReference>,
) -> Result<JsFunction> {
    env.create_function_from_closure("factstrStreamCallbackRejected", move |ctx: CallContext| {
        let _ =
            complete_stream_callback_with_reference(ctx.env.raw(), complete_ref.as_ref(), false);
        ctx.env.get_undefined()
    })
}

fn create_stream_completion_callback(
    env: Env,
    completion_sender: Arc<Mutex<Option<SyncSender<bool>>>>,
) -> Result<JsFunction> {
    env.create_function_from_closure("factstrStreamCallbackComplete", move |ctx: CallContext| {
        let callback_succeeded = if ctx.length == 0 {
            false
        } else {
            match ctx.get::<JsUnknown>(0) {
                Ok(value) => callback_completed_successfully(ctx.env.raw(), value).unwrap_or(false),
                Err(_) => false,
            }
        };

        if let Ok(mut completion_sender) = completion_sender.lock() {
            if let Some(completion_sender) = completion_sender.take() {
                let _ = completion_sender.send(callback_succeeded);
            }
        }

        ctx.env.get_undefined()
    })
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
