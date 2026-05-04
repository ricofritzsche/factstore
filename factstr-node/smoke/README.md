# factstr-node smoke test

This smoke test verifies the published `@factstr/factstr-node` package surface.
It is package verification, not a public example application.

The smoke suite proves:

- `FactstrMemoryStore`
- `FactstrSqliteStore`
- `append`
- `query`
- `appendIf` success and explicit conflict
- `min_sequence_number`
- `last_returned_sequence_number`
- `current_context_version`
- `streamAll`
- `streamTo`
- `streamAllDurable`
- `streamToDurable`
- SQLite database creation and reopen
- direct-native durable callback failure behavior

The smoke test intentionally registers failing callbacks. Callback failure logs are
expected during the run.

## Install dependencies

```bash
cd factstr-node
npm --prefix smoke run build
```

## Run the smoke build

```bash
cd factstr-node
npm --prefix smoke run build
```

## Run the smoke test

```bash
cd factstr-node
npm --prefix smoke run smoke
```

## Run after a packed local install

```bash
cd factstr-node
npm run build
npm run pack:local
npm run pack:prebuilt:current
npm_config_cache=/tmp/factstr-npm-cache npm --prefix smoke run install:packed
npm --prefix smoke run build
npm --prefix smoke run smoke
```
