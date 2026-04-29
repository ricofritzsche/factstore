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

## Local Testing

Build the native package:

```bash
cd factstr-node
npm run build
```

Create the main npm tarball:

```bash
cd factstr-node
npm run pack:local
```

Build and pack the current platform prebuilt package:

```bash
cd factstr-node
npm run pack:prebuilt:current
```

Install those tarballs into the smoke consumer:

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

## Distribution Model

`factstr-node` is intended to ship prebuilt native binaries.

- the public npm package is `factstr-node`
- platform packages carry the native `.node` binary for one OS and CPU combination
- npm installs the matching platform package through `optionalDependencies`
- `factstr-node` loads that platform package first and falls back to a locally built binary only for developer workflows in this repository

Local Rust builds remain a developer fallback. They are not the primary consumer installation path.

## Supported Targets

Current prebuilt targets wired by this repository:

- `darwin-arm64`
- `darwin-x64`
- `linux-x64-gnu`
- `win32-x64-msvc`

The local `.npmrc` files keep npm cache data inside `factstr-node/.npm-cache` so these commands do not rely on a writable global npm cache.
