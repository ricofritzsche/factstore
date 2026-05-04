# factstr-sqlite

`factstr-sqlite` is the embedded SQLite store implementation for the shared [`factstr`](https://crates.io/crates/factstr) contract.

Use this crate when an application needs local persistence without running a separate database server.

## What it implements

`factstr-sqlite` implements the shared `factstr::EventStore` contract, including:

- `append`
- `query`
- `append_if`
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

## When to use it

Use `factstr-sqlite` when:

- you want persistent facts in a local SQLite database
- you want the same contract as the in-memory and PostgreSQL stores
- you want durable stream cursor state to survive restart

## Store behavior and boundaries

- Committed facts are stored in SQLite.
- Durable stream cursors are stored in SQLite and survive restart.
- Durable replay depends on persisted append-batch history.
- Older databases without contiguous append-batch history are rejected for durable replay instead of being silently backfilled.

## Add to `Cargo.toml`

```toml
[dependencies]
factstr = "0.3.1"
factstr-sqlite = "0.3.1"
serde_json = "1"
```

## Minimal example

```rust
use factstr::{EventQuery, EventStore, NewEvent};
use factstr_sqlite::SqliteStore;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = SqliteStore::open("./factstr.sqlite")?;

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
- [`factstr-postgres`](https://crates.io/crates/factstr-postgres): PostgreSQL-backed store

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0
