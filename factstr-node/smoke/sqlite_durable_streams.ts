import {
  type DurableStream,
  type EventQuery,
  type EventRecord,
  FactstrSqliteStore,
} from '@factstr/factstr-node';
import { assert } from './smoke_assert';
import { createSqliteDatabasePaths } from './sqlite_database_paths';
import { waitForCondition, waitForNoNewDeliveries } from './wait_for_delivery';

export async function runSqliteDurableStreamsSmoke(): Promise<void> {
  const sqlitePaths = createSqliteDatabasePaths('factstr-node-smoke-sqlite-durable-');
  await exerciseSqliteStreamAllDurable(sqlitePaths.databasePath('durable-all'));
  await exerciseSqliteStreamToDurable(sqlitePaths.databasePath('durable-filtered'));
}

async function exerciseSqliteStreamAllDurable(sqliteDatabasePath: string): Promise<void> {
  const durableStream: DurableStream = { name: 'sqlite-durable-all' };
  const initialBatches: EventRecord[][] = [];

  let store = new FactstrSqliteStore(sqliteDatabasePath);
  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'sqlite-durable-1', owner: 'Rico' },
    },
  ]);
  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'sqlite-durable-1', amount: 25 },
    },
  ]);

  const durableSubscription = store.streamAllDurable(durableStream, (events) => {
    initialBatches.push(events);
  });

  await waitForCondition('sqlite durable all replay', () => initialBatches.length === 2);
  assert(initialBatches[0].length === 1, 'sqlite durable all replay should preserve the first committed batch');
  assert(initialBatches[0][0].sequence_number === 1n, 'sqlite durable all replay first sequence should be 1n');
  assert(initialBatches[1].length === 1, 'sqlite durable all replay should preserve the second committed batch');
  assert(initialBatches[1][0].sequence_number === 2n, 'sqlite durable all replay second sequence should be 2n');

  store.append([
    {
      event_type: 'account-tagged',
      payload: { accountId: 'sqlite-durable-1', tag: 'vip' },
    },
  ]);
  await waitForCondition('sqlite durable all future delivery', () => initialBatches.length === 3);
  assert(initialBatches[2][0].sequence_number === 3n, 'sqlite durable all should continue with sequence 3n');

  const conflictResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId: 'sqlite-durable-1', amount: 50 },
      },
    ],
    {
      filters: [
        {
          event_types: ['account-opened'],
          payload_predicates: [{ accountId: 'sqlite-durable-1' }],
        },
      ],
    },
    0n,
  );
  assert(conflictResult.append_result == null, 'sqlite durable all appendIf conflict should return no append result');
  assert(conflictResult.conflict != null, 'sqlite durable all appendIf conflict should remain explicit');
  await waitForNoNewDeliveries('sqlite durable all conflict deliveries', () => initialBatches.length, 3);

  durableSubscription.unsubscribe();
  durableSubscription.unsubscribe();

  store = new FactstrSqliteStore(sqliteDatabasePath);
  const resumedBatches: EventRecord[][] = [];
  const resumedSubscription = store.streamAllDurable(durableStream, (events) => {
    resumedBatches.push(events);
  });

  await waitForNoNewDeliveries('sqlite durable all replay after reopen', () => resumedBatches.length, 0);

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'sqlite-durable-1', amount: 40 },
    },
  ]);
  await waitForCondition('sqlite durable all resumed future delivery', () => resumedBatches.length === 1);
  assert(resumedBatches[0][0].sequence_number === 4n, 'sqlite durable all should resume after the stored cursor');

  resumedSubscription.unsubscribe();
}

async function exerciseSqliteStreamToDurable(sqliteDatabasePath: string): Promise<void> {
  const durableStream: DurableStream = { name: 'sqlite-durable-filtered' };
  const filteredQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
      },
    ],
  };

  let store = new FactstrSqliteStore(sqliteDatabasePath);
  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'sqlite-filtered-1', owner: 'Rico' },
    },
  ]);
  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'sqlite-filtered-1', amount: 10 },
    },
  ]);

  const filteredBatches: EventRecord[][] = [];
  const filteredSubscription = store.streamToDurable(durableStream, filteredQuery, (events) => {
    filteredBatches.push(events);
  });

  await waitForCondition('sqlite durable filtered replay', () => filteredBatches.length === 1);
  assert(filteredBatches[0].length === 1, 'sqlite durable filtered replay should deliver one matching event');
  assert(filteredBatches[0][0].sequence_number === 2n, 'sqlite durable filtered replay should skip non-matching history');

  store.append([
    {
      event_type: 'account-tagged',
      payload: { accountId: 'sqlite-filtered-1', tag: 'vip' },
    },
    {
      event_type: 'money-deposited',
      payload: { accountId: 'sqlite-filtered-1', amount: 20 },
    },
  ]);
  await waitForCondition('sqlite durable filtered future batch', () => filteredBatches.length === 2);
  assert(filteredBatches[1].length === 1, 'sqlite durable filtered future batch should keep only matching events');
  assert(filteredBatches[1][0].sequence_number === 4n, 'sqlite durable filtered future batch should preserve committed order');

  const conflictResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId: 'sqlite-filtered-1', amount: 30 },
      },
    ],
    {
      filters: [
        {
          event_types: ['account-opened'],
        },
      ],
    },
    0n,
  );
  assert(conflictResult.append_result == null, 'sqlite durable filtered appendIf conflict should return no append result');
  assert(conflictResult.conflict != null, 'sqlite durable filtered appendIf conflict should remain explicit');
  await waitForNoNewDeliveries('sqlite durable filtered conflict deliveries', () => filteredBatches.length, 2);

  filteredSubscription.unsubscribe();
  filteredSubscription.unsubscribe();

  store = new FactstrSqliteStore(sqliteDatabasePath);
  const resumedFilteredBatches: EventRecord[][] = [];
  const resumedFilteredSubscription = store.streamToDurable(
    durableStream,
    filteredQuery,
    (events) => {
      resumedFilteredBatches.push(events);
    },
  );

  await waitForNoNewDeliveries('sqlite durable filtered replay after reopen', () => resumedFilteredBatches.length, 0);

  store.append([
    {
      event_type: 'account-tagged',
      payload: { accountId: 'sqlite-filtered-1', tag: 'gold' },
    },
  ]);
  await waitForNoNewDeliveries('sqlite durable filtered non-matching future batch', () => resumedFilteredBatches.length, 0);

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'sqlite-filtered-1', amount: 40 },
    },
  ]);
  await waitForCondition('sqlite durable filtered resumed future delivery', () => resumedFilteredBatches.length === 1);
  assert(resumedFilteredBatches[0].length === 1, 'sqlite durable filtered resumed delivery should still filter correctly');
  assert(resumedFilteredBatches[0][0].sequence_number === 6n, 'sqlite durable filtered resumed delivery should continue after the stored cursor');

  resumedFilteredSubscription.unsubscribe();
}
