# factstr-node

`factstr-node` is the TypeScript and Node package surface for the current FACTSTR language adapter.

Current scope:

- `FactstrMemoryStore`
- `append(events)`
- `query(query)`
- `appendIf(events, query, expectedContextVersion)`

This package is memory-backed only in this step.

It does not expose:

- SQLite
- PostgreSQL
- streams
- durable streams
- transport behavior

Sequence and context values are exposed as `bigint` in TypeScript so the Rust `u64` meanings stay lossless.

## Local Package Proof

Build the native package:

```bash
cd factstr-node
npm run build
```

Create the npm tarball:

```bash
cd factstr-node
npm run pack:local
```

Install that tarball into the smoke consumer:

```bash
cd factstr-node/smoke
npm run install:packed
```

Build and run the TypeScript smoke test:

```bash
cd factstr-node/smoke
npm run build
npm run smoke
```

Successful output includes:

```text
factstr-node TypeScript smoke test passed
```

The local `.npmrc` files keep npm cache data inside `factstr-node/.npm-cache` so these commands do not rely on a writable global npm cache.
