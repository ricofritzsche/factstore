use crate::{
    AppendIfResult, AppendResult, DurableStream, EventQuery, EventStreamSubscription, NewEvent,
    QueryResult, sequence_number_value::option_bigint_to_u64,
    stream_callback::handle_stream_from_js_function,
    stream_error::napi_error_from_event_store_error,
};
use factstr::{EventStore, EventStoreError};
use factstr_sqlite::SqliteStore;
use napi::bindgen_prelude::BigInt;
use napi::bindgen_prelude::Result;
use napi::{Env, JsFunction};
use napi_derive::napi;

#[napi]
pub struct FactstrSqliteStore {
    sqlite_store: SqliteStore,
}

#[napi]
impl FactstrSqliteStore {
    #[napi(constructor)]
    pub fn new(database_path: String) -> Result<Self> {
        let sqlite_store = SqliteStore::open(&database_path).map_err(|error| {
            napi_error_from_event_store_error(EventStoreError::BackendFailure {
                message: error.to_string(),
            })
        })?;

        Ok(Self { sqlite_store })
    }

    #[napi]
    pub fn append(&self, events: Vec<NewEvent>) -> Result<AppendResult> {
        let interop_events = events
            .into_iter()
            .map(NewEvent::into_interop)
            .collect::<Vec<_>>();
        let new_events = interop_events
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let append_result = self
            .sqlite_store
            .append(new_events)
            .map_err(napi_error_from_event_store_error)?;

        Ok(factstr_interop::InteropAppendResult::from(append_result).into())
    }

    #[napi]
    pub fn query(&self, query: EventQuery) -> Result<QueryResult> {
        let interop_query = query.into_interop()?;
        let event_query = interop_query.into();
        let query_result = self
            .sqlite_store
            .query(&event_query)
            .map_err(napi_error_from_event_store_error)?;

        Ok(factstr_interop::InteropQueryResult::from(query_result).into())
    }

    #[napi(js_name = "appendIf")]
    pub fn append_if(
        &self,
        events: Vec<NewEvent>,
        query: EventQuery,
        expected_context_version: Option<BigInt>,
    ) -> Result<AppendIfResult> {
        let interop_events = events
            .into_iter()
            .map(NewEvent::into_interop)
            .collect::<Vec<_>>();
        let new_events = interop_events
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let interop_query = query.into_interop()?;
        let event_query = interop_query.into();
        let expected_context_version = option_bigint_to_u64(expected_context_version)?;

        match self
            .sqlite_store
            .append_if(new_events, &event_query, expected_context_version)
        {
            Ok(append_result) => Ok(AppendIfResult {
                append_result: Some(
                    factstr_interop::InteropAppendResult::from(append_result).into(),
                ),
                conflict: None,
            }),
            Err(EventStoreError::ConditionalAppendConflict { expected, actual }) => {
                Ok(AppendIfResult {
                    append_result: None,
                    conflict: Some(
                        factstr_interop::InteropConditionalAppendConflict::new(expected, actual)
                            .into(),
                    ),
                })
            }
            Err(error) => Err(napi_error_from_event_store_error(error)),
        }
    }

    #[napi(js_name = "streamAll")]
    pub fn stream_all(&self, env: Env, handle: JsFunction) -> Result<EventStreamSubscription> {
        let stream_handle = handle_stream_from_js_function(env, handle)?;
        let event_stream = self
            .sqlite_store
            .stream_all(stream_handle)
            .map_err(napi_error_from_event_store_error)?;

        Ok(EventStreamSubscription::new(event_stream))
    }

    #[napi(js_name = "streamTo")]
    pub fn stream_to(
        &self,
        query: EventQuery,
        env: Env,
        handle: JsFunction,
    ) -> Result<EventStreamSubscription> {
        let stream_handle = handle_stream_from_js_function(env, handle)?;
        let interop_query = query.into_interop()?;
        let event_query = interop_query.into();
        let event_stream = self
            .sqlite_store
            .stream_to(&event_query, stream_handle)
            .map_err(napi_error_from_event_store_error)?;

        Ok(EventStreamSubscription::new(event_stream))
    }

    #[napi(js_name = "streamAllDurable")]
    pub fn stream_all_durable(
        &self,
        env: Env,
        durable_stream: DurableStream,
        handle: JsFunction,
    ) -> Result<EventStreamSubscription> {
        let stream_handle = handle_stream_from_js_function(env, handle)?;
        let factstr_durable_stream = durable_stream.into_factstr();
        let event_stream = self
            .sqlite_store
            .stream_all_durable(&factstr_durable_stream, stream_handle)
            .map_err(napi_error_from_event_store_error)?;

        Ok(EventStreamSubscription::new(event_stream))
    }

    #[napi(js_name = "streamToDurable")]
    pub fn stream_to_durable(
        &self,
        env: Env,
        durable_stream: DurableStream,
        query: EventQuery,
        handle: JsFunction,
    ) -> Result<EventStreamSubscription> {
        let stream_handle = handle_stream_from_js_function(env, handle)?;
        let factstr_durable_stream = durable_stream.into_factstr();
        let interop_query = query.into_interop()?;
        let event_query = interop_query.into();
        let event_stream = self
            .sqlite_store
            .stream_to_durable(&factstr_durable_stream, &event_query, stream_handle)
            .map_err(napi_error_from_event_store_error)?;

        Ok(EventStreamSubscription::new(event_stream))
    }
}
