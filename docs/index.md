# FACTSTR

[![crates.io](https://img.shields.io/crates/v/factstr?label=Cargo&color=8A2BE2)](https://crates.io/crates/factstr) [![npm](https://img.shields.io/npm/v/%40factstr%2Ffactstr-node?label=npm&color=CB3837)](https://www.npmjs.com/package/@factstr/factstr-node)

FACTSTR is a Rust event store built around append-only facts, command context consistency, committed-batch streams, and durable replay.

It helps applications keep decisions local to the facts that matter, append new facts safely, and build query models from committed batches without introducing aggregate-first structure.

## What You Can Use

FACTSTR currently provides:

- a Rust contract crate: `factstr`
- store implementations for Memory, SQLite, and PostgreSQL
- live streams with committed-batch delivery
- durable streams for replay and catch-up
- Node.js bindings and TypeScript types through `@factstr/factstr-node`

## Why It Matters

Use FACTSTR when you want:

- append-only facts instead of mutable domain state as the source of truth
- command context consistency instead of aggregate-centric locking
- explicit sequence numbers and ordered reads
- projections that can replay and continue after restart
- an embedded SQLite option before introducing external infrastructure
- a PostgreSQL option when the application already runs PostgreSQL

## Install

### Rust

```toml
[dependencies]
factstr = "0.3"
factstr-sqlite = "0.3"
```

Memory and PostgreSQL stores are also available through `factstr-memory` and `factstr-postgres`.

### Node.js and TypeScript

```bash
npm install @factstr/factstr-node
```

The Node.js bindings currently expose the Memory, SQLite, and PostgreSQL stores. PostgreSQL requires a database URL.

## Start Here

- [Getting Started](getting-started.md)
- [Node and TypeScript](node-typescript.md)
- [Core Concepts](core-concepts.md)
- [Stores](stores.md)
- [Streams](streams.md)
- [Reference](reference.md)
