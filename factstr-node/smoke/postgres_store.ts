import {
  type DurableStream,
  type EventQuery,
  type EventRecord,
  FactstrPostgresStore,
} from '@factstr/factstr-node';
import { assert } from './smoke_assert';
import { waitForCondition, waitForNoNewDeliveries } from './wait_for_delivery';

declare const process: {
  env?: Record<string, string | undefined>;
};

function postgresDatabaseUrl(): string | null {
  const env = process.env ?? {};
  return env.FACTSTR_NODE_POSTGRES_DATABASE_URL ?? env.DATABASE_URL ?? null;
}

function uniquePostgresSmokeId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function batchContainsEvent(
  batches: EventRecord[][],
  predicate: (eventRecord: EventRecord) => boolean,
): boolean {
  return batches.some((batch) => batch.some(predicate));
}

function findBatch(
  batches: EventRecord[][],
  predicate: (batch: EventRecord[]) => boolean,
): EventRecord[] | null {
  return batches.find(predicate) ?? null;
}

export async function runPostgresStoreSmoke(): Promise<void> {
  const databaseUrl = postgresDatabaseUrl();
  if (databaseUrl == null) {
    console.log(
      'Skipping PostgreSQL smoke: set FACTSTR_NODE_POSTGRES_DATABASE_URL or DATABASE_URL.',
    );
    return;
  }

  const accountId = uniquePostgresSmokeId('postgres-account');
  const durableAllStream: DurableStream = { name: uniquePostgresSmokeId('postgres-durable-all') };
  const durableFilteredStream: DurableStream = {
    name: uniquePostgresSmokeId('postgres-durable-filtered'),
  };
  const store = new FactstrPostgresStore(databaseUrl);
  const allBatches: EventRecord[][] = [];
  const filteredBatches: EventRecord[][] = [];

  const depositQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
        payload_predicates: [{ accountId }],
      },
    ],
  };

  const openResult = store.append([
    {
      event_type: 'account-opened',
      payload: { accountId, owner: 'Rico' },
    },
  ]);
  assert(openResult.first_sequence_number > 0n, 'postgres append should return a positive first sequence number');
  assert(openResult.last_sequence_number >= openResult.first_sequence_number, 'postgres append should preserve sequence ordering');

  const openedQuery = store.query({
    filters: [
      {
        event_types: ['account-opened'],
        payload_predicates: [{ accountId }],
      },
    ],
  });
  assert(openedQuery.event_records.length === 1, 'postgres query should return the appended account-opened fact');
  const currentContextVersion = openedQuery.current_context_version ?? null;
  assert(currentContextVersion != null, 'postgres query should expose current context version');

  const appendIfSuccess = store.appendIf(
    [
      {
        event_type: 'account-tagged',
        payload: { accountId, tag: 'vip' },
      },
    ],
    {
      filters: [
        {
          event_types: ['account-opened'],
          payload_predicates: [{ accountId }],
        },
      ],
    },
    currentContextVersion,
  );
  assert(appendIfSuccess.append_result != null, 'postgres appendIf should succeed with the current context version');
  assert(appendIfSuccess.conflict == null, 'postgres appendIf success should not expose a conflict');

  const appendIfConflict = store.appendIf(
    [
      {
        event_type: 'account-tagged',
        payload: { accountId, tag: 'duplicate' },
      },
    ],
    {
      filters: [
        {
          event_types: ['account-opened'],
          payload_predicates: [{ accountId }],
        },
      ],
    },
    0n,
  );
  assert(appendIfConflict.append_result == null, 'postgres appendIf conflict should not return an append result');
  assert(appendIfConflict.conflict != null, 'postgres appendIf conflict should stay explicit');

  const streamAllSubscription = store.streamAll((events) => {
    allBatches.push(events);
  });
  const streamToSubscription = store.streamTo(depositQuery, (events) => {
    filteredBatches.push(events);
  });

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 25 },
    },
    {
      event_type: 'account-tagged',
      payload: { accountId, tag: 'gold' },
    },
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 40 },
    },
  ]);

  await waitForCondition(
    'postgres streamAll future delivery',
    () => batchContainsEvent(
      allBatches,
      (eventRecord) =>
        eventRecord.event_type === 'money-deposited'
        && (eventRecord.payload as { accountId?: string; amount?: number }).accountId === accountId
        && (eventRecord.payload as { amount?: number }).amount === 40,
    ),
  );
  await waitForCondition(
    'postgres streamTo filtered delivery',
    () => filteredBatches.some(
      (batch) =>
        batch.length === 2
        && batch.every(
          (eventRecord) =>
            eventRecord.event_type === 'money-deposited'
            && (eventRecord.payload as { accountId?: string }).accountId === accountId,
        ),
    ),
  );
  const matchingAllBatch = findBatch(
    allBatches,
    (batch) =>
      batch.length === 3
      && batch.some(
        (eventRecord) =>
          eventRecord.event_type === 'account-tagged'
          && (eventRecord.payload as { accountId?: string; tag?: string }).accountId === accountId
          && (eventRecord.payload as { tag?: string }).tag === 'gold',
      ),
  );
  assert(matchingAllBatch != null, 'postgres streamAll should deliver the committed mixed batch');
  const matchingFilteredBatch = findBatch(
    filteredBatches,
    (batch) =>
      batch.length === 2
      && batch.every(
        (eventRecord) =>
          eventRecord.event_type === 'money-deposited'
          && (eventRecord.payload as { accountId?: string }).accountId === accountId,
      ),
  );
  assert(matchingFilteredBatch != null, 'postgres streamTo should keep only matching events');

  streamAllSubscription.unsubscribe();
  streamToSubscription.unsubscribe();

  const durableAllReplayBatches: EventRecord[][] = [];
  const durableAllSubscription = store.streamAllDurable(durableAllStream, (events) => {
    durableAllReplayBatches.push(events);
  });
  await waitForCondition(
    'postgres durable all replay',
    () => batchContainsEvent(
      durableAllReplayBatches,
      (eventRecord) =>
        eventRecord.event_type === 'account-opened'
        && (eventRecord.payload as { accountId?: string }).accountId === accountId,
    ),
  );
  assert(
    batchContainsEvent(
      durableAllReplayBatches,
      (eventRecord) =>
        eventRecord.event_type === 'account-opened'
        && (eventRecord.payload as { accountId?: string }).accountId === accountId,
    ),
    'postgres durable replay should include the historical account-opened fact',
  );
  assert(
    durableAllReplayBatches.some(
      (batch) =>
        batch.length === 3
        && batch.some(
          (eventRecord) =>
            eventRecord.event_type === 'account-tagged'
            && (eventRecord.payload as { accountId?: string; tag?: string }).accountId === accountId
            && (eventRecord.payload as { tag?: string }).tag === 'gold',
        ),
    ),
    'postgres durable replay should preserve the committed mixed batch',
  );

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 50 },
    },
  ]);
  await waitForCondition(
    'postgres durable all future delivery',
    () => batchContainsEvent(
      durableAllReplayBatches,
      (eventRecord) =>
        eventRecord.event_type === 'money-deposited'
        && (eventRecord.payload as { accountId?: string; amount?: number }).accountId === accountId
        && (eventRecord.payload as { amount?: number }).amount === 50,
    ),
  );
  durableAllSubscription.unsubscribe();

  const falseReturnDurableStream: DurableStream = {
    name: uniquePostgresSmokeId('postgres-durable-false-return'),
  };
  const falseReturnCursorSubscription = store.streamAllDurable(
    falseReturnDurableStream,
    () => {},
  );
  falseReturnCursorSubscription.unsubscribe();

  const falseReturnBatches: EventRecord[][] = [];
  const falseReturnSubscription = store.streamAllDurable(
    falseReturnDurableStream,
    (events) => {
      falseReturnBatches.push(events);
      return false;
    },
  );

  store.append([
    {
      event_type: 'account-note-added',
      payload: { accountId, note: 'delivery should retry' },
    },
  ]);
  await waitForCondition(
    'postgres durable false return delivery',
    () => batchContainsEvent(
      falseReturnBatches,
      (eventRecord) =>
        eventRecord.event_type === 'account-note-added'
        && (eventRecord.payload as { accountId?: string }).accountId === accountId,
    ),
  );
  falseReturnSubscription.unsubscribe();

  const recoveredFalseReturnBatches: EventRecord[][] = [];
  const recoveredFalseReturnSubscription = store.streamAllDurable(
    falseReturnDurableStream,
    (events) => {
      recoveredFalseReturnBatches.push(events);
    },
  );
  await waitForCondition(
    'postgres durable false return replay',
    () => batchContainsEvent(
      recoveredFalseReturnBatches,
      (eventRecord) =>
        eventRecord.event_type === 'account-note-added'
        && (eventRecord.payload as { accountId?: string }).accountId === accountId,
    ),
  );
  assert(
    recoveredFalseReturnBatches[0].some(
      (eventRecord) =>
        eventRecord.event_type === 'account-note-added'
        && (eventRecord.payload as { accountId?: string }).accountId === accountId,
    ),
    'postgres durable false return should replay the undelivered batch',
  );
  recoveredFalseReturnSubscription.unsubscribe();

  const durableFilteredReplayBatches: EventRecord[][] = [];
  const durableFilteredSubscription = store.streamToDurable(
    durableFilteredStream,
    depositQuery,
    (events) => {
      durableFilteredReplayBatches.push(events);
    },
  );
  await waitForCondition(
    'postgres durable filtered replay',
    () =>
      batchContainsEvent(
        durableFilteredReplayBatches,
        (eventRecord) =>
          eventRecord.event_type === 'money-deposited'
          && (eventRecord.payload as { accountId?: string; amount?: number }).accountId === accountId
          && (eventRecord.payload as { amount?: number }).amount === 50,
      ),
  );
  assert(
    durableFilteredReplayBatches[0].every((eventRecord) => eventRecord.event_type === 'money-deposited'),
    'postgres durable filtered replay should keep only matching facts',
  );

  store.append([
    {
      event_type: 'account-tagged',
      payload: { accountId, tag: 'platinum' },
    },
  ]);
  await waitForNoNewDeliveries(
    'postgres durable filtered non-matching future delivery',
    () => durableFilteredReplayBatches.length,
    2,
  );

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 75 },
    },
  ]);
  await waitForCondition(
    'postgres durable filtered future delivery',
    () => durableFilteredReplayBatches.length === 3,
  );
  assert(
    durableFilteredReplayBatches[2].length === 1,
    'postgres durable filtered future delivery should still return only matching facts',
  );
  durableFilteredSubscription.unsubscribe();
}
