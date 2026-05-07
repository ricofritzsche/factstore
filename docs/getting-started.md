# Getting Started

This page shows the shortest path to using FACTSTR from a Rust project or from Node.js and TypeScript.

[![crates.io](https://img.shields.io/crates/v/factstr?label=Cargo&color=8A2BE2)](https://crates.io/crates/factstr)
[![npm](https://img.shields.io/npm/v/%40factstr%2Ffactstr-node?label=npm&color=CB3837)](https://www.npmjs.com/package/@factstr/factstr-node)

## Use FACTSTR From Rust

For normal Rust projects, use the published crates from crates.io.

Core contract only:

```toml
[dependencies]
factstr = "0.4.1"
```

Memory store:

```toml
[dependencies]
factstr = "0.4.1"
factstr-memory = "0.4.1"
```

SQLite store:

```toml
[dependencies]
factstr = "0.4.1"
factstr-sqlite = "0.4.1"
```

PostgreSQL store:

```toml
[dependencies]
factstr = "0.4.1"
factstr-postgres = "0.4.1"
```

Choose a store based on how you want to run FACTSTR:

- use `factstr-memory` for tests and local experiments
- use `factstr-sqlite` for embedded persistence without a separate database server
- use `factstr-postgres` when the application already runs PostgreSQL

## Minimal Rust Example

This is the shortest concrete Rust example using the in-memory store.

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

That example shows the basic contract shape:

- create a store
- append one fact
- query it back

See [Examples](examples.md) for repository examples that go further into projections and feature slices.

## Use FACTSTR From Node.js and TypeScript

`@factstr/factstr-node` provides Node.js bindings and TypeScript types for FACTSTR.

Install it:

```bash
npm install @factstr/factstr-node
```

The current package surface includes:

- `FactstrMemoryStore`
- `FactstrSqliteStore`
- `FactstrPostgresStore`
- `append`
- `query`
- `appendIf`
- `streamAll`
- `streamTo`
- `streamAllDurable`
- `streamToDurable`

Current boundaries:

- PostgreSQL requires a PostgreSQL database URL
- transport behavior is not exposed

See [Node and TypeScript](node-typescript.md) for the current package examples and boundaries.

## Next Pages

- [Core Concepts](core-concepts.md)
- [Stores](stores.md)
- [Streams](streams.md)
- [Node and TypeScript](node-typescript.md)
- [Reference](reference.md)

## Work On The Repository

Use the GitHub repository when you want to contribute to FACTSTR or test unreleased changes.

Start from the repository README for the current contributor and release workflow context.
