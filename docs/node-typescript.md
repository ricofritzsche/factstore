# Node and TypeScript

`@factstr/factstr-node` is the current published Node and TypeScript package for FACTSTR.

It provides Node.js bindings and TypeScript types for FACTSTR. The package exposes the Memory, SQLite, and PostgreSQL stores from the Rust implementation without reimplementing FACTSTR semantics in TypeScript.

## Install

```bash
npm install @factstr/factstr-node
```

## Current Stores

- `FactstrMemoryStore`
- `FactstrSqliteStore`
- `FactstrPostgresStore`

## Current API

- `append`
- `query`
- `appendIf`
- `streamAll`
- `streamTo`
- `streamAllDurable`
- `streamToDurable`
- `DurableStream`
- `EventStreamSubscription`

## Append And Query

```ts
import {
  type EventQuery,
  type NewEvent,
  FactstrMemoryStore,
  FactstrPostgresStore,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const memoryStore = new FactstrMemoryStore();
const sqliteStore = new FactstrSqliteStore('./factstr.sqlite');
const postgresStore = new FactstrPostgresStore(process.env.DATABASE_URL!);

const event: NewEvent = {
  event_type: 'item-added',
  payload: { sku: 'ABC-123', quantity: 1 },
};

memoryStore.append([event]);
sqliteStore.append([event]);
postgresStore.append([event]);

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
console.log(result.last_returned_sequence_number);
console.log(result.current_context_version);
```

This example keeps the current public package shape explicit:

- events use `event_type` and `payload`
- `query(...)` returns `event_records`
- each `EventRecord` includes `occurred_at`
- `last_returned_sequence_number` and `current_context_version` stay distinct

## Conditional Append Example

```ts
import {
  type AppendIfResult,
  type EventQuery,
  type NewEvent,
  FactstrMemoryStore,
} from '@factstr/factstr-node';

const store = new FactstrMemoryStore();

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

`appendIf(...)` checks whether the relevant command context changed before the new facts are committed.

## Live Streams

```ts
import {
  FactstrSqliteStore,
  type EventQuery,
} from '@factstr/factstr-node';

const store = new FactstrSqliteStore('./factstr.sqlite');

const reservationFacts: EventQuery = {
  filters: [
    {
      event_types: ['item-reserved', 'reservation-cancelled'],
    },
  ],
};

const allSubscription = store.streamAll((events) => {
  console.log('all committed batch', events);
});

const filteredSubscription = store.streamTo(reservationFacts, (events) => {
  console.log('reservation batch', events);
});

allSubscription.unsubscribe();
filteredSubscription.unsubscribe();
```

`streamAll(...)` observes future committed batches. `streamTo(...)` observes future committed facts that match the given query.

## Durable Streams

```ts
import {
  type DurableStream,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

const store = new FactstrSqliteStore('./factstr.sqlite');
const durableStream: DurableStream = { name: 'inventory-projector' };

const subscription = store.streamAllDurable(durableStream, (events) => {
  console.log('replayed or live batch', events);
});

subscription.unsubscribe();
```

`streamAllDurable(...)` and `streamToDurable(...)` replay committed facts strictly after the stored durable cursor and then continue with future committed delivery.

## BigInt

Sequence and context values use `bigint` so Rust `u64` meanings stay lossless in TypeScript.

`occurred_at` is exposed as an RFC 3339 string on each returned event record.

## Store Durability Boundary

- `FactstrMemoryStore` keeps durable stream state only for the lifetime of one store instance.
- `FactstrSqliteStore` persists facts and durable stream state in SQLite, so the same durable stream name can resume across reopening the same database path.
- `FactstrPostgresStore` persists facts and durable stream state in PostgreSQL, so the same durable stream name can resume as long as the database state is retained.

## Not Included Yet

- transport behavior
