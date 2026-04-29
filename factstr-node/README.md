# factstr-node

`@factstr/factstr-node` is the TypeScript and Node package surface for the current FACTSTR language adapter.

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

- the public npm package is `@factstr/factstr-node`
- platform packages carry the native `.node` binary for one OS and CPU combination
- npm installs the matching platform package through `optionalDependencies`
- `factstr-node` loads that platform package first and falls back to a locally built binary only for developer workflows in this repository

Local Rust builds remain a developer fallback. They are not the primary consumer installation path.

## Release Model

`@factstr/factstr-node` is the publishable npm product in this repository.

- publishing runs in GitHub Actions
- npm trusted publishing is the intended steady-state path
- the prebuilt support packages under `npm/*` exist only to carry platform-native `.node` artifacts
- the Rust `factstr-node` crate and `factstr-interop` are workspace support crates, not crates.io products

The Node adapter line stays version-locked across:

- `factstr-node/package.json`
- `factstr-node/Cargo.toml`
- `factstr-node/npm/*/package.json`

The repository CI checks that these versions stay in sync before release workflows run.

Normal release flow does not publish from a developer machine.

## Supported Targets

Current prebuilt targets wired by this repository:

- `darwin-arm64`
- `darwin-x64`
- `linux-x64-gnu`
- `win32-x64-msvc`

The local `.npmrc` files keep npm cache data inside `factstr-node/.npm-cache` so these commands do not rely on a writable global npm cache.
