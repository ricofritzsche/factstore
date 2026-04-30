# Examples

These examples are the shortest concrete code paths for the current FACTSTR API.

## Rust Examples In This Repository

## Basic Memory Store

Source:

- [`examples/basic-memory/src/main.rs`](https://github.com/ricofritzsche/factstr/blob/main/examples/basic-memory/src/main.rs)

Run it:

```bash
cargo run --manifest-path examples/basic-memory/Cargo.toml
```

What it proves:

- create a `MemoryStore`
- append one committed batch
- query the stored facts
- inspect:
  - returned `EventRecord` values
  - `last_returned_sequence_number`
  - `current_context_version`

## Account Projection

Source:

- [`examples/account-projection/src/main.rs`](https://github.com/ricofritzsche/factstr/blob/main/examples/account-projection/src/main.rs)

Run it:

```bash
cargo run --manifest-path examples/account-projection/Cargo.toml
```

What it proves:

- define a feature-local read model
- register `stream_to(&EventQuery, handle)` for only the relevant facts
- append one committed batch where some facts match and others do not
- update the read model from the one delivered committed batch
- keep unrelated facts out of that projection by contract

This is the main feature-slice story in the current repository: the feature owns its own query model, receives only relevant future facts, and updates that model from committed batches.

These examples stay on the memory store because it is the fastest way to understand the current contract without database setup.

## Bank Slices CLI

Source:

- [`examples/bank-slices-cli/src/main.rs`](https://github.com/ricofritzsche/factstr/blob/main/examples/bank-slices-cli/src/main.rs)
- [`examples/bank-slices-cli/README.md`](https://github.com/ricofritzsche/factstr/blob/main/examples/bank-slices-cli/README.md)

Run it:

```bash
cargo run --manifest-path examples/bank-slices-cli/Cargo.toml
```

What it proves:

- SQLite-backed feature slices can stay independent without a central `Event` enum or aggregate
- write-side slices remain explicit `*Command` types
- read-side slices are explicit queries such as `BalanceQuery` and `MovementHistoryQuery`
- the `fetch_balance` slice owns a real `stream_to(...)` registration and keeps its local read model current from committed batches
- the `fetch_movement_history` slice reads facts directly and stays separate from `fetch_balance`

This is the production-like example for the current repository: persisted facts in SQLite, local write-side rule checks, and a real stream-driven read model without introducing a shared domain layer.

## Node and TypeScript Package Examples

The current Node and TypeScript examples live with the published package surface rather than as a separate repository example directory.

Use:

- [`docs/node-typescript.md`](node-typescript.md) for the current package examples
- [`factstr-node/NPM.md`](https://github.com/ricofritzsche/factstr/blob/main/factstr-node/NPM.md) for the npm-facing package documentation source
- [`factstr-node/smoke/index.ts`](https://github.com/ricofritzsche/factstr/blob/main/factstr-node/smoke/index.ts) for the small package smoke consumer used in packaging verification

Current Node and TypeScript examples are intentionally narrow:

- `FactstrMemoryStore`
- `append`
- `query`
- `appendIf`

They do not currently demonstrate SQLite, PostgreSQL, streams, durable streams, or transport behavior because the published package does not expose those features yet.
