# factstr-memory

`factstr-memory` is the in-memory store implementation for the shared [`factstr`](https://crates.io/crates/factstr) contract.

Use this crate when you want FACTSTR behavior without persistence, for example in tests, examples, local development, or semantic reference scenarios.

## What it implements

`factstr-memory` implements the shared `factstr::EventStore` contract, including:

- `append`
- `query`
- `append_if`
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

## When to use it

Use `factstr-memory` when:

- you want an in-process store with no external dependencies
- you need fast semantic tests
- you want to exercise the same contract as the persistent store crates

## Store behavior and boundaries

- Events are kept in process memory only.
- Recreating the store starts with an empty event log.
- Durable streams are durable only within one `MemoryStore` instance.
- Recreating the store also resets in-memory durable stream cursor state.

## Add to `Cargo.toml`

```toml
[dependencies]
factstr = "0.3.1"
factstr-memory = "0.3.1"
serde_json = "1"
```

## Minimal example

```rust
use factstr::{EventQuery, EventStore, NewEvent};
use factstr_memory::MemoryStore;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::new();

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
- [`factstr-sqlite`](https://crates.io/crates/factstr-sqlite): embedded persistent store
- [`factstr-postgres`](https://crates.io/crates/factstr-postgres): PostgreSQL-backed store

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0
