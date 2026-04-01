# Stores

FACTSTR currently includes three store implementations.

## Memory Store

The memory store is the simplest runtime implementation and the semantic reference for the repository.

It is useful for:

- local development
- tests
- direct reasoning about the contract

It implements:

- append
- query
- conditional append
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

Its durable stream boundary is explicit:

- durable stream cursor state is kept only for the lifetime of one `MemoryStore` instance
- replay/catch-up is real within that store instance
- recreating the store starts with a fresh in-memory durable cursor state

## SQLite Store

The SQLite store is the embedded persistent implementation.

It is useful for:

- local persistence without external infrastructure
- feature-local durable replay/catch-up
- validating the shared contract against an embedded store

It implements:

- append
- query
- conditional append
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

Operationally, it differs from the other stores by:

- persisting events and durable stream cursors in SQLite
- replaying committed batches from stored cursors before switching to future committed delivery
- bounding durable replay correctness on persisted `append_batches` history

Operationally, it persists committed events and durable stream cursors across process restarts.
Durable replay still depends on persisted `append_batches` history, so older databases created before that history existed are rejected for durable replay instead of being backfilled automatically.

## PostgreSQL Store

The PostgreSQL store implements the same shared contract on top of PostgreSQL using SQLx.

It is useful for:

- teams that already operate PostgreSQL
- persistence with familiar database tooling
- validating the shared contract against a real database-backed implementation

It implements:

- append
- query
- conditional append
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

Operationally, it differs from the memory store by:

- persisting committed events and durable stream cursors in PostgreSQL
- using SQL transactions and indexes
- using an internal worker thread so the synchronous store API remains safe to call from inside a running Tokio runtime
- rejecting durable replay on older databases that do not have contiguous `append_batches` history

## Shared Semantics Across Stores

Even though the mechanics differ, the intended observable behavior remains shared:

- append/query semantics
- conditional append semantics
- sequence allocation guarantees
- explicit query result meanings
- future committed stream behavior

## Shared Durable-Stream Status

Durable streams are implemented across all three stores.

- Memory keeps durable stream state for the lifetime of one store instance
- SQLite persists durable stream cursors and replay state across restart, with an explicit `append_batches` history boundary for older databases
- PostgreSQL persists durable stream cursors and replay state across restart, with an explicit `append_batches` history boundary for older databases
