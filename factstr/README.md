# factstr

`factstr` is the shared Rust contract crate for FACTSTR.

It defines the public event store contract and core types. It does not implement
a store by itself.

Store implementations live in separate crates:

- `factstr-memory`
- `factstr-sqlite`
- `factstr-postgres`

## Core operations

The core contract currently includes:

- `append`
- `query`
- `append_if`
- `stream_all`
- `stream_to`
- `stream_all_durable`
- `stream_to_durable`

## Core types

The public contract includes:

- `NewEvent`
- `EventRecord`
- `EventQuery`
- `EventFilter`
- `QueryResult`
- `AppendResult`
- `DurableStream`
- `EventStream`
- `EventStore`
- `EventStoreError`

Related stream handler types are also part of the crate:

- `HandleStream`
- `StreamHandlerError`

## Semantics

Important contract semantics:

- facts are append-only
- sequence numbers are global and monotonically increasing
- one committed batch receives one consecutive sequence range
- `min_sequence_number` is a read cursor only
- `current_context_version` describes the full matching conflict context
- `append_if` checks the full matching context
- durable streams resume strictly after the stored cursor

## Related crates

- `factstr-memory` for the in-memory implementation
- `factstr-sqlite` for the SQLite implementation
- `factstr-postgres` for the PostgreSQL implementation
