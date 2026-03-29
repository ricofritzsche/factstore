use factstore::{
    AppendResult, EventQuery, EventRecord, EventStore, EventStoreError, NewEvent, QueryResult,
};
use sqlx::{
    PgPool, Postgres, QueryBuilder, Row, Transaction,
    postgres::{PgPoolOptions, PgRow},
};
use tokio::runtime::{Builder, Runtime};

use crate::query_sql::push_query_conditions;

pub struct PostgresStore {
    runtime: Runtime,
    pool: PgPool,
}

impl PostgresStore {
    pub fn connect(connection_string: &str) -> Result<Self, sqlx::Error> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build");
        let pool = runtime.block_on(async {
            PgPoolOptions::new()
                .max_connections(1)
                .connect(connection_string)
                .await
        })?;

        runtime.block_on(initialize_schema(&pool))?;

        Ok(Self { runtime, pool })
    }

    fn query_inner(&self, event_query: &EventQuery) -> Result<QueryResult, sqlx::Error> {
        self.runtime.block_on(async {
            let current_context_version = current_context_version(&self.pool, event_query).await?;

            let mut query_builder: QueryBuilder<'_, Postgres> =
                QueryBuilder::new("SELECT sequence_number, event_type, payload FROM events");
            push_query_conditions(&mut query_builder, event_query, true);
            query_builder.push(" ORDER BY sequence_number ASC");

            let event_records = query_builder
                .build()
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(event_record_from_row)
                .collect::<Vec<_>>();

            let last_returned_sequence_number = event_records
                .last()
                .map(|event_record| event_record.sequence_number);

            Ok(QueryResult {
                event_records,
                last_returned_sequence_number,
                current_context_version,
            })
        })
    }

    fn append_inner(&self, new_events: Vec<NewEvent>) -> Result<AppendResult, sqlx::Error> {
        self.runtime.block_on(async {
            let mut transaction = self.pool.begin().await?;
            lock_events_table(&mut transaction).await?;
            let append_result = append_records(&mut transaction, new_events).await?;
            transaction.commit().await?;
            Ok(append_result)
        })
    }

    fn append_if_inner(
        &self,
        new_events: Vec<NewEvent>,
        context_query: &EventQuery,
        expected_context_version: Option<u64>,
    ) -> Result<Result<AppendResult, EventStoreError>, sqlx::Error> {
        self.runtime.block_on(async {
            let mut transaction = self.pool.begin().await?;
            lock_events_table(&mut transaction).await?;

            let actual_context_version =
                current_context_version_in_transaction(&mut transaction, context_query).await?;

            if actual_context_version != expected_context_version {
                transaction.rollback().await?;
                return Ok(Err(EventStoreError::ConditionalAppendConflict {
                    expected: expected_context_version,
                    actual: actual_context_version,
                }));
            }

            let append_result = append_records(&mut transaction, new_events).await?;
            transaction.commit().await?;

            Ok(Ok(append_result))
        })
    }

    fn backend_failure(error: sqlx::Error) -> EventStoreError {
        EventStoreError::BackendFailure {
            message: error.to_string(),
        }
    }
}

impl EventStore for PostgresStore {
    fn query(&self, event_query: &EventQuery) -> Result<QueryResult, EventStoreError> {
        self.query_inner(event_query).map_err(Self::backend_failure)
    }

    fn append(&self, new_events: Vec<NewEvent>) -> Result<AppendResult, EventStoreError> {
        if new_events.is_empty() {
            return Err(EventStoreError::EmptyAppend);
        }

        self.append_inner(new_events).map_err(Self::backend_failure)
    }

    fn append_if(
        &self,
        new_events: Vec<NewEvent>,
        context_query: &EventQuery,
        expected_context_version: Option<u64>,
    ) -> Result<AppendResult, EventStoreError> {
        if new_events.is_empty() {
            return Err(EventStoreError::EmptyAppend);
        }

        self.append_if_inner(new_events, context_query, expected_context_version)
            .map_err(Self::backend_failure)?
    }
}

async fn initialize_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    // The TypeScript baseline uses BIGSERIAL for sequence_number. This store keeps explicit
    // sequence assignment instead so committed batches retain one consecutive range without
    // gaps from failed appends or rolled-back conditional appends.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            sequence_number BIGINT PRIMARY KEY,
            occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            event_type TEXT NOT NULL,
            payload JSONB NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_occurred_at ON events(occurred_at)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_payload_gin ON events USING gin(payload)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn lock_events_table(transaction: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query("LOCK TABLE events IN EXCLUSIVE MODE")
        .execute(transaction.as_mut())
        .await?;

    Ok(())
}

async fn append_records(
    transaction: &mut Transaction<'_, Postgres>,
    new_events: Vec<NewEvent>,
) -> Result<AppendResult, sqlx::Error> {
    let committed_count = new_events.len() as u64;
    let first_sequence_number =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(sequence_number), 0) + 1 FROM events")
            .fetch_one(transaction.as_mut())
            .await? as u64;
    let last_sequence_number = first_sequence_number + committed_count - 1;

    for (offset, new_event) in new_events.into_iter().enumerate() {
        let sequence_number = first_sequence_number + offset as u64;
        sqlx::query(
            "INSERT INTO events (sequence_number, event_type, payload)
             VALUES ($1, $2, $3)",
        )
        .bind(sequence_number as i64)
        .bind(new_event.event_type)
        .bind(new_event.payload)
        .execute(transaction.as_mut())
        .await?;
    }

    Ok(AppendResult {
        first_sequence_number,
        last_sequence_number,
        committed_count,
    })
}

async fn current_context_version(
    pool: &PgPool,
    event_query: &EventQuery,
) -> Result<Option<u64>, sqlx::Error> {
    let mut query_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("SELECT MAX(sequence_number) FROM events");
    push_query_conditions(&mut query_builder, event_query, false);

    Ok(query_builder
        .build_query_scalar::<Option<i64>>()
        .fetch_one(pool)
        .await?
        .map(|sequence_number| sequence_number as u64))
}

async fn current_context_version_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    event_query: &EventQuery,
) -> Result<Option<u64>, sqlx::Error> {
    let mut query_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("SELECT MAX(sequence_number) FROM events");
    push_query_conditions(&mut query_builder, event_query, false);

    Ok(query_builder
        .build_query_scalar::<Option<i64>>()
        .fetch_one(transaction.as_mut())
        .await?
        .map(|sequence_number| sequence_number as u64))
}

fn event_record_from_row(row: PgRow) -> EventRecord {
    EventRecord {
        sequence_number: row.get::<i64, _>("sequence_number") as u64,
        event_type: row.get("event_type"),
        payload: row.get("payload"),
    }
}
