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
- append_if
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
- reproducible local verification with a real database file

It implements:

- append
- query
- append_if
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

Operationally, it differs from the other stores by:

- persisting events and durable stream cursors in SQLite
- replaying committed batches from stored cursors before switching to future committed delivery
- storing `append_batches` rows only for committed multi-event appends

Operationally, it persists committed events and durable stream cursors across process restarts. Durable replay reconstructs committed batch boundaries from persisted `append_batches` rows when they exist, and treats missing rows as single-event appends.

For guidance on when SQLite is a good fit and when it is not, see [SQLite Store: What It Is For, and When Not to Use It](sqlite.md).

## PostgreSQL Store

The PostgreSQL store implements the same shared contract on top of PostgreSQL using SQLx.

It is useful for:

- teams that already operate PostgreSQL
- persistence with familiar database tooling
- validating the shared contract against a real database-backed implementation

It implements:

- append
- query
- append_if
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

Operationally, it differs from the memory store by:

- persisting committed events and durable stream cursors in PostgreSQL
- using SQL transactions and indexes
- using an internal worker thread so the synchronous store API remains safe to call from inside a running Tokio runtime
- storing `append_batches` rows only for committed multi-event appends
- treating missing `append_batches` rows as single-event appends during durable replay

## Shared Semantics Across Stores

Even though the mechanics differ, the intended observable behavior remains shared:

- append/query semantics
- conditional append semantics
- sequence allocation guarantees
- explicit query result meanings
- future committed stream behavior
- durable replay/catch-up semantics
- durable cursor safety

## Shared Durable-Stream Status

Durable streams are implemented across all three stores.

- Memory implements durable streams within one `MemoryStore` instance only
- SQLite implements durable streams with persisted cursors and replay across restart
- PostgreSQL implements durable streams with persisted cursors and replay across restart

Shared reusable durable-stream conformance now exists across the stores.
Remaining store-specific tests prove only store-local boundaries:

- Memory durable state is instance-lifetime only
- SQLite stores append-boundary metadata only for committed multi-event appends
- PostgreSQL stores append-boundary metadata only for committed multi-event appends
