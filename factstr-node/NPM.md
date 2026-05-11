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
  type EventQuery,
  type NewEvent,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const store = new FactstrSqliteStore('./factstr.sqlite');

const event: NewEvent = {
  event_type: 'item-added',
  payload: { sku: 'ABC-123', quantity: 1 },
};

store.append([event]);

const query: EventQuery = {
  filters: [
    {
      event_types: ['item-added'],
    },
  ],
};

const result = store.query(query);

console.log(result.event_records[0]?.occurred_at);
console.log(result.event_records[0]?.payload);
```

## PostgreSQL

### Existing Database

```ts
import { FactstrPostgresStore } from '@factstr/factstr-node';

const store = new FactstrPostgresStore(process.env.DATABASE_URL!);
```

Use this when the target database already exists.

### Create Database If Missing

```ts
import { FactstrPostgresStore } from '@factstr/factstr-node';

const store = FactstrPostgresStore.bootstrap({
  serverUrl: 'postgres://postgres:postgres@localhost:5432/postgres',
  databaseName: 'factstr_demo',
});
```

Use this when the PostgreSQL server already exists but the target database may not exist yet.

## Live Streams

`streamAll(...)` and `streamTo(...)` register synchronously and return `EventStreamSubscription`.

They observe only future committed batches after registration becomes active.

Callbacks receive one committed batch as an array of `EventRecord` values.

`streamTo` applies the same query matching semantics as `query` and delivers only the matching facts from each future committed batch.

Node stream callbacks may return:

- `void`
- `boolean`
- `Promise<void>`
- `Promise<boolean>`

Callback results mean:

- `undefined` or `void`: success
- `true`: success
- `false`: failure
- resolved `undefined` or `true`: success
- resolved `false`: failure
- rejected `Promise`: failure
- synchronous throw: failure

Live callback failure is isolated from append success. A successful append stays committed even if a live callback throws, returns `false`, or rejects later.

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

## Live vs Durable Stream Registration

`streamAll(...)` and `streamTo(...)` register live streams. They observe only future committed batches after registration becomes active. Registration is synchronous because no replay happens during registration.

```ts
async function updateProjection(events: EventRecord[]): Promise<void> {
  console.log('persist projection updates', events);
}

const subscription = store.streamAll(async (events) => {
  await updateProjection(events);
});
```

The callback may return a `Promise`, but append success is not rolled back if the callback fails.

`streamAllDurable(...)` and `streamToDurable(...)` register durable streams. They may replay existing committed batches before returning the subscription. Registration is asynchronous because FACTSTR waits for replay callback success before advancing the durable cursor.

```ts
const subscription = await store.streamAllDurable(
  { name: 'inventory-projector' },
  async (events) => {
    await updateProjection(events);
  },
);
```

For durable streams, the cursor advances only after the callback succeeds. If the callback throws, returns `false`, rejects, or resolves to `false`, the cursor does not advance.

API summary:

```ts
streamAll(handle): EventStreamSubscription
streamTo(query, handle): EventStreamSubscription

streamAllDurable(name, handle): Promise<EventStreamSubscription>
streamToDurable(name, query, handle): Promise<EventStreamSubscription>
```

## Durable Streams

`streamAllDurable(...)` and `streamToDurable(...)` register asynchronously and return `Promise<EventStreamSubscription>`.

Durable registration must be awaited because it may replay committed facts during registration, and replay waits for callback completion before advancing the durable cursor.

After registration completes, durable streams replay facts strictly after the stored durable cursor and then continue with future committed delivery.

Callbacks receive one committed batch as an array of `EventRecord` values.

`FactstrMemoryStore` keeps durable stream state only for the lifetime of the current memory store instance.

`FactstrSqliteStore` persists durable stream state in SQLite, so the same durable stream name can resume across reopening the same database path.

`FactstrPostgresStore` persists facts and durable stream state in PostgreSQL, so the same durable stream name can resume as long as the database state is retained.

```ts
import {
  type DurableStream,
  type EventQuery,
  type EventRecord,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const store = new FactstrSqliteStore('./factstr.sqlite');
const durableStream: DurableStream = { name: 'inventory-projector' };

async function updateProjection(events: EventRecord[]): Promise<void> {
  console.log('persist projection updates', events);
}

const subscription = await store.streamAllDurable(
  durableStream,
  async (events) => {
    await updateProjection(events);
  },
);

store.append([
  {
    event_type: 'item-added',
    payload: { sku: 'ABC-123', quantity: 1 },
  },
]);

subscription.unsubscribe();
```

```ts
const query: EventQuery = {
  filters: [
    {
      event_types: ['item-added'],
    },
  ],
};

const filteredSubscription = await store.streamToDurable(
  durableStream,
  query,
  async (events) => {
    await updateProjection(events);
  },
);

filteredSubscription.unsubscribe();
```

Durable cursor advancement depends on callback success:

- successful sync callback: cursor advances
- successful async callback: cursor advances after the returned `Promise` resolves successfully
- thrown error, resolved `false`, or rejected `Promise`: cursor does not advance

Retrying the same durable stream name replays from the last successfully processed sequence.

For future durable deliveries, append success is still not rolled back by callback failure, but durable cursor advancement for that delivery still depends on callback success.

If a durable callback returns a `Promise` that never settles, that delivery stays in flight. `unsubscribe()` stops future deliveries, but it does not convert an already in-flight callback into success. The durable cursor advances only if that in-flight callback eventually succeeds.

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
