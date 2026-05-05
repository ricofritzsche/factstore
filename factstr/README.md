# factstr

`factstr` is the shared Rust contract crate for FACTSTR.

FACTSTR is an event store built around append-only facts, global sequencing, streams, durable streams, and command context consistency.

This crate defines the public contract and the core types. It does not store events by itself. Runtime stores live in separate crates.

## What this crate provides

The `factstr` crate defines the common API used by all FACTSTR store implementations.

It includes:

* event input and event record types
* event queries and filters
* append results
* query results
* conditional append conflicts
* live stream contracts
* durable stream contracts
* the `EventStore` trait

The purpose of this crate is to keep the observable behavior of different stores aligned. Memory, SQLite, and PostgreSQL can use different internal mechanisms, but they implement the same public contract.

## Store crates

Use this crate together with one of the runtime store implementations:

* `factstr-memory` for tests, examples, and local development
* `factstr-sqlite` for embedded persistent usage without a separate database server
* `factstr-postgres` for PostgreSQL-backed persistence

The core crate stays independent of those implementations.

## Add to `Cargo.toml`

```toml
[dependencies]
factstr = "0.3"
```

Use the GitHub repository when you want to work on FACTSTR itself or test unreleased changes before a crates.io release.

## Core idea

FACTSTR treats events as facts in one append-only log.

Each committed event receives a global sequence number. A committed batch receives one consecutive sequence range. Reads return events in ascending sequence order.

Consistency is checked against the facts relevant to a command. A command reads the current version of its context, then appends new facts only if that context has not changed.

That means consistency is not tied to aggregate roots or stream-per-entity ownership. The relevant context is described by an `EventQuery`.

## Main operations

The shared contract supports:

* `append`
* `query`
* `append_if`
* `stream_all`
* `stream_to`
* `stream_all_durable`
* `stream_to_durable`

## Important semantics

FACTSTR keeps several meanings explicit because they matter for correctness.

### Append

Appending a batch produces an `AppendResult` with:

* `first_sequence_number`
* `last_sequence_number`
* `committed_count`

A successful batch is committed as one consecutive range.

### Query

A query returns matching event records and two separate sequence meanings:

* `last_returned_sequence_number`
* `current_context_version`

`last_returned_sequence_number` describes the events returned by the read.

`current_context_version` describes the full matching context used for consistency checks.

Those values are intentionally separate.

### `min_sequence_number`

`min_sequence_number` is a read cursor only.

It limits which rows are returned by a query, but it does not shrink the consistency context used by `append_if`.

### Conditional append

`append_if` appends only when the expected context version still matches the current version of the full matching context.

If the context changed, the store returns a conditional append conflict instead of appending a partial batch.

### Streams

Live streams observe future committed batches.

Durable streams replay committed batches after a stored cursor and then continue with future delivery. Durable cursors must not advance past undelivered committed facts.

## Contract types

The central public types are:

* `NewEvent`
* `EventRecord`
* `EventFilter`
* `EventQuery`
* `QueryResult`
* `AppendResult`
* `DurableStream`
* `EventStream`
* `EventStore`
* `EventStoreError`

## Example: query and conditional append

This example uses the contract shape. A concrete store such as `factstr-memory` or `factstr-sqlite` provides the actual implementation.

```rust
use factstr::{EventQuery, EventFilter, NewEvent};

fn reservation_context(item_id: &str, window_key: &str) -> EventQuery {
    EventQuery {
        filters: Some(vec![
            EventFilter {
                event_types: Some(vec!["item-reserved".to_string()]),
                payload_predicates: Some(vec![
                    serde_json::json!({
                        "item_id": item_id,
                        "window_key": window_key
                    })
                ]),
            }
        ]),
        min_sequence_number: None,
    }
}

let context = reservation_context("camera-1", "2026-05-10-morning");

let event = NewEvent {
    event_type: "item-reserved".to_string(),
    payload: serde_json::json!({
        "reservation_id": "res-1",
        "item_id": "camera-1",
        "window_key": "2026-05-10-morning"
    }),
};
```

The command first queries the context, reads `current_context_version`, and then calls `append_if` on a concrete store implementation.

## Choosing a store

Use `factstr-memory` when the process lifetime is enough and persistence is not needed.

Use `factstr-sqlite` when the application should keep facts locally without operating a separate database server.

Use `factstr-postgres` when the application should use PostgreSQL as the persistent store.

## Related packages

Rust store crates:

* `factstr-memory`
* `factstr-sqlite`
* `factstr-postgres`

Node and TypeScript package:

* `@factstr/factstr-node`

## License

Licensed under either of:

* MIT
* Apache-2.0
