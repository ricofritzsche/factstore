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

## Add to `Cargo.toml`

```toml
[dependencies]
factstr = "0.3.1"
factstr-postgres = "0.3.1"
serde_json = "1"
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

## Related crates

- [`factstr`](https://crates.io/crates/factstr): shared contract and core types
- [`factstr-memory`](https://crates.io/crates/factstr-memory): in-memory store
- [`factstr-sqlite`](https://crates.io/crates/factstr-sqlite): embedded persistent store

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0
