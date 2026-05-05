# FACTSTR

FACTSTR is a Rust event store built around append-only facts, command context consistency, committed-batch streams, and durable replay.

It is designed to start small: append facts, read the facts relevant to a decision, append conditionally when the command context has not changed, and keep query models current from committed batches.

[![crates.io](https://img.shields.io/crates/v/factstr?label=Cargo&color=8A2BE2)](https://crates.io/crates/factstr)
[![npm](https://img.shields.io/npm/v/%40factstr%2Ffactstr-node?label=npm&color=CB3837)](https://www.npmjs.com/package/@factstr/factstr-node)

## Install

### Rust

Core contract:

```toml
[dependencies]
factstr = "0.3"
```

Memory:

```toml
[dependencies]
factstr = "0.3"
factstr-memory = "0.3"
```

SQLite:

```toml
[dependencies]
factstr = "0.3"
factstr-sqlite = "0.3"
```

PostgreSQL:

```toml
[dependencies]
factstr = "0.3"
factstr-postgres = "0.3"
```

### Node.js and TypeScript

```bash
npm install @factstr/factstr-node
```

## What FACTSTR Gives You

- append-only event records with global sequence numbers
- conditional append through command context consistency
- ordered queries with explicit read cursor and context version meanings
- live streams that deliver committed batches after persistence succeeds
- durable streams that replay after a stored cursor and then continue live
- Memory, SQLite, and PostgreSQL store implementations behind one Rust contract
- Node.js bindings with TypeScript types for Memory and SQLite

## Rust Crates

- `factstr`: shared Rust contract crate
- `factstr-memory`: in-memory store for tests, examples, and local development
- `factstr-sqlite`: embedded persistent SQLite store
- `factstr-postgres`: PostgreSQL-backed store
- `factstr-conformance`: internal semantic test support
- `factstr-interop`: internal interop boundary for bindings
- Rust crate part of `factstr-node`: internal build substrate for the npm package

For normal Rust projects, install the contract crate plus one store crate.

## Node.js Bindings

`@factstr/factstr-node` provides Node.js bindings and TypeScript types for FACTSTR. It exposes the Memory and SQLite stores from the Rust implementation without reimplementing FACTSTR semantics in TypeScript.

Current package surface:

- `FactstrMemoryStore`
- `FactstrSqliteStore`
- `append`
- `query`
- `appendIf`
- `streamAll`
- `streamTo`
- `streamAllDurable`
- `streamToDurable`

Current boundaries:

- PostgreSQL support is not exposed through Node.js yet
- transport behavior is not exposed

## Documentation

- Website: [factstr.com](https://factstr.com)
- Getting Started: [docs/getting-started.md](docs/getting-started.md)
- Core Concepts: [docs/core-concepts.md](docs/core-concepts.md)
- Stores: [docs/stores.md](docs/stores.md)
- Streams: [docs/streams.md](docs/streams.md)
- SQLite guidance: [docs/sqlite.md](docs/sqlite.md)
- Node and TypeScript: [docs/node-typescript.md](docs/node-typescript.md)
- Reference: [docs/reference.md](docs/reference.md)

## Repository

Use the published crates and npm package for normal projects. Clone this repository when you want to work on FACTSTR itself, inspect examples, or test unreleased changes.

```bash
cargo check
cargo test --workspace --exclude factstr-postgres
```

Rust crates and the Node.js package are published by separate GitHub Actions workflows.

## License

Licensed under either of:

- MIT
- Apache-2.0
