# factstr-postgres

`factstr-postgres` is the PostgreSQL-backed store implementation for the shared [`factstr`](https://crates.io/crates/factstr) contract.

Use this crate when an application already operates PostgreSQL and wants FACTSTR persistence there.

## What it implements

`factstr-postgres` implements the shared `factstr::EventStore` contract, including:

- `append`
- `query`
- `append_if`
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

## When to use it

Use `factstr-postgres` when:

- you want FACTSTR backed by PostgreSQL
- your application already operates PostgreSQL
- you want durable stream cursor state to persist in PostgreSQL

## Store behavior and boundaries

- Committed facts and durable stream cursors are stored in PostgreSQL.
- SQL transactions and indexes are used internally.
- An internal worker thread keeps the synchronous store API safe to call from inside a running Tokio runtime.
- Durable replay depends on persisted append-batch history.
- Older databases without contiguous append-batch history are rejected for durable replay instead of being silently backfilled.

Integration tests for this crate require a PostgreSQL database.

## Connection paths

`factstr-postgres` exposes two explicit PostgreSQL setup paths:

- `PostgresStore::connect(database_url)`: connect to an existing database, then initialize or validate the FACTSTR schema inside that database.
- `PostgresStore::bootstrap(options)`: connect to an existing PostgreSQL server, create the target database if it is missing, derive the target database connection URL from `server_url + database_name`, then continue through the normal `connect` path.

`connect` keeps production-style usage explicit: the database lifecycle stays outside the store, but FACTSTR schema setup stays local to `factstr-postgres`.

`bootstrap` is intended for local development, examples, demos, and reference implementations that should start from an existing PostgreSQL server without requiring the client to create the target database first. The PostgreSQL server itself remains external, and the credentials in `server_url` still need permission to inspect `pg_database` and create the target database.

Bootstrap intentionally supports only simple identifier-style database names that match `[A-Za-z_][A-Za-z0-9_]*`. Names with whitespace, punctuation, path-special characters, or other quoted PostgreSQL identifier forms are rejected before FACTSTR checks `pg_database`, creates the database, or derives the target connection URL.

## Add to `Cargo.toml`

```toml
[dependencies]
factstr = "0.4.1"
factstr-postgres = "0.4.1"
```

## Minimal example

```rust
use factstr::{EventQuery, EventStore, NewEvent};
use factstr_postgres::PostgresStore;
use serde_json::json;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let store = PostgresStore::connect(&database_url)?;

    store.append(vec![NewEvent {
        event_type: "item-added".to_owned(),
        payload: json!({ "sku": "ABC-123", "quantity": 1 }),
    }])?;

    let result = store.query(&EventQuery::all())?;
    assert_eq!(result.event_records.len(), 1);

    Ok(())
}
```

## Bootstrap example

```rust
use factstr::{EventQuery, EventStore};
use factstr_postgres::{PostgresBootstrapOptions, PostgresStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgresStore::bootstrap(PostgresBootstrapOptions {
        server_url: "postgres://postgres:postgres@localhost:5432/postgres".to_owned(),
        database_name: "factstr_demo".to_owned(),
    })?;

    let result = store.query(&EventQuery::all())?;
    assert!(result.event_records.is_empty());

    Ok(())
}
```

## Related crates

- [`factstr`](https://crates.io/crates/factstr): shared contract and core types
- [`factstr-memory`](https://crates.io/crates/factstr-memory): in-memory store
- [`factstr-sqlite`](https://crates.io/crates/factstr-sqlite): embedded persistent store

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0
