# @factstr/factstr-node

`@factstr/factstr-node` provides Node.js bindings and TypeScript types for FACTSTR.

It exposes the Memory, SQLite, and PostgreSQL stores from the Rust implementation without reimplementing FACTSTR semantics in TypeScript. The current package surface stays small and explicit:

- `FactstrMemoryStore`
- `FactstrSqliteStore`
- `FactstrPostgresStore`
- `FactstrPostgresStore.bootstrap`
- `append`
- `query`
- `appendIf`
- `streamAll`
- `streamTo`
- `streamAllDurable`
- `streamToDurable`

## Current Scope

Current scope is intentionally narrow:

- memory-backed, SQLite-backed, and PostgreSQL-backed store access
- explicit append, query, conditional-append, live-stream, and durable-stream behavior
- non-durable live streams for future committed batches
- durable streams that replay after the stored cursor and then continue with future committed delivery
- TypeScript-friendly package surface

Not included yet:

- transport behavior

## Install

```bash
npm install @factstr/factstr-node
```

SQLite and PostgreSQL support are included in the same package. `FactstrMemoryStore` is memory-backed, `FactstrSqliteStore` is SQLite-backed and persistent, and `FactstrPostgresStore` supports two explicit PostgreSQL paths:

- `new FactstrPostgresStore(databaseUrl)`: the target PostgreSQL database already exists, and FACTSTR initializes or validates the schema it owns inside that database.
- `FactstrPostgresStore.bootstrap({ serverUrl, databaseName })`: FACTSTR starts from an existing PostgreSQL server connection, creates the target database when missing, and then returns a ready store through the normal PostgreSQL connect path.

Bootstrap still requires an existing PostgreSQL server. FACTSTR does not provision or run PostgreSQL itself, and the configured credentials must have permission to inspect `pg_database` and create the target database. Bootstrap database names are intentionally limited to `[A-Za-z_][A-Za-z0-9_]*`.

## Supported Platforms

Current prebuilt targets:

- `darwin-arm64`
- `darwin-x64`
- `linux-x64-gnu`
- `win32-x64-msvc`

## Quick Start

```ts
import {
  type DurableStream,
  type EventQuery,
  type NewEvent,
  FactstrMemoryStore,
  FactstrPostgresStore,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const memoryStore = new FactstrMemoryStore();
const sqliteStore = new FactstrSqliteStore('./factstr.sqlite');
const postgresStore = new FactstrPostgresStore(process.env.DATABASE_URL!);
const bootstrappedPostgresStore = FactstrPostgresStore.bootstrap({
  serverUrl: 'postgres://postgres:postgres@localhost:5432/postgres',
  databaseName: 'factstr_demo',
});

const event: NewEvent = {
  event_type: 'item-added',
  payload: { sku: 'ABC-123', quantity: 1 },
};

memoryStore.append([event]);
sqliteStore.append([event]);
postgresStore.append([event]);
bootstrappedPostgresStore.append([event]);

const query: EventQuery = {
  filters: [
    {
      event_types: ['item-added'],
    },
  ],
};

const result = sqliteStore.query(query);

console.log(result.event_records[0]?.occurred_at);
console.log(result.event_records[0]?.payload);
```

## Live Streams

`streamAll` and `streamTo` observe only future committed batches after registration becomes active.

Callbacks receive one committed batch as an array of `EventRecord` values.

`streamTo` applies the same query matching semantics as `query` and delivers only the matching facts from each future committed batch.

`unsubscribe()` stops future deliveries for that stream.

```ts
import {
  type EventQuery,
  FactstrMemoryStore,
} from '@factstr/factstr-node';

const store = new FactstrMemoryStore();

const streamAllSubscription = store.streamAll((events) => {
  console.log('committed batch', events);
});

const filteredQuery: EventQuery = {
  filters: [
    {
      event_types: ['item-added'],
    },
  ],
};

const streamToSubscription = store.streamTo(filteredQuery, (events) => {
  console.log('filtered committed batch', events);
});

streamAllSubscription.unsubscribe();
streamToSubscription.unsubscribe();
```

## Durable Streams

`streamAllDurable` and `streamToDurable` replay facts strictly after the stored durable cursor and then continue with future committed delivery.

Callbacks receive one committed batch as an array of `EventRecord` values.

`FactstrMemoryStore` keeps durable stream state only for the lifetime of the current memory store instance.

`FactstrSqliteStore` persists durable stream state in SQLite, so the same durable stream name can resume across reopening the same database path.

`FactstrPostgresStore` persists facts and durable stream state in PostgreSQL, so the same durable stream name can resume as long as the database state is retained.

```ts
import {
  type DurableStream,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const store = new FactstrSqliteStore('./factstr.sqlite');
const durableStream: DurableStream = { name: 'inventory-projector' };

const subscription = store.streamAllDurable(durableStream, (events) => {
  console.log('durable committed batch', events);
});

store.append([
  {
    event_type: 'item-added',
    payload: { sku: 'ABC-123', quantity: 1 },
  },
]);

subscription.unsubscribe();
```

## Conditional Append

`appendIf` checks whether the relevant command context has changed before appending new facts.

```ts
import {
  type AppendIfResult,
  type EventQuery,
  type NewEvent,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const store = new FactstrSqliteStore('./factstr.sqlite');

const contextQuery: EventQuery = {
  filters: [
    {
      event_types: ['item-added'],
    },
  ],
};

const context = store.query(contextQuery);

const nextEvent: NewEvent = {
  event_type: 'item-added',
  payload: { sku: 'ABC-123', quantity: 1 },
};

const outcome: AppendIfResult = store.appendIf(
  [nextEvent],
  contextQuery,
  context.current_context_version,
);

if (outcome.conflict) {
  console.log('conditional append conflict', outcome.conflict);
} else {
  console.log('append succeeded', outcome.append_result);
}
```

## BigInt

Sequence and context values are exposed as `bigint` so FACTSTR's Rust `u64` meanings stay lossless in TypeScript.

`occurred_at` is exposed as an RFC 3339 string on each returned `EventRecord`.

## Binding Boundaries

`FactstrMemoryStore` is memory-backed and process-local.

`FactstrSqliteStore` is SQLite-backed and persistent at the database path you pass to the constructor.

`FactstrPostgresStore` is PostgreSQL-backed.

Use the constructor when the target database already exists.

Use `FactstrPostgresStore.bootstrap({ serverUrl, databaseName })` when an existing PostgreSQL server should create the target database first. Bootstrap database names must match `[A-Za-z_][A-Za-z0-9_]*`.

All three Node.js store bindings expose `append`, `query`, `appendIf`, `streamAll`, `streamTo`, `streamAllDurable`, and `streamToDurable`.

Live streams are future-only.

Durable streams replay after the stored cursor and then continue with future delivery.

Transport behavior is still not exposed through the Node.js bindings yet.

## Docs and Source

- [https://factstr.com](https://factstr.com)
- [https://github.com/ricofritzsche/factstr](https://github.com/ricofritzsche/factstr)
