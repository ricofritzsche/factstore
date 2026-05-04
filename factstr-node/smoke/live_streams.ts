import {
  type AppendIfResult,
  type EventQuery,
  type EventRecord,
  FactstrMemoryStore,
  FactstrSqliteStore,
} from '@factstr/factstr-node';
import { assert } from './smoke_assert';
import { createSqliteDatabasePaths } from './sqlite_database_paths';
import { waitForCondition, waitForNoNewDeliveries } from './wait_for_delivery';

type LiveStreamStore = Pick<
  FactstrMemoryStore,
  'append' | 'appendIf' | 'query' | 'streamAll' | 'streamTo'
>;

export async function runLiveStreamsSmoke(): Promise<void> {
  await exerciseLiveStore('memory', new FactstrMemoryStore(), 'memory-live-1');

  const sqlitePaths = createSqliteDatabasePaths('factstr-node-smoke-live-streams-');
  await exerciseLiveStore(
    'sqlite',
    new FactstrSqliteStore(sqlitePaths.databasePath('live-streams')),
    'sqlite-live-1',
  );
}

async function exerciseLiveStore(
  storeName: string,
  store: LiveStreamStore,
  accountId: string,
): Promise<void> {
  const streamAllBatches: EventRecord[][] = [];
  const filteredBatches: EventRecord[][] = [];
  const healthyBatches: EventRecord[][] = [];
  const thrownBatches: EventRecord[][] = [];

  const streamAllSubscription = store.streamAll((events) => {
    streamAllBatches.push(events);
  });
  const failingSubscription = store.streamAll((events) => {
    thrownBatches.push(events);
    throw new Error(`${storeName} failing callback should not break append`);
  });

  const appendResult = store.append([
    {
      event_type: 'account-opened',
      payload: { accountId, owner: 'Rico' },
    },
  ]);
  assert(appendResult.first_sequence_number === 1n, `${storeName} expected first sequence number 1n`);
  assert(appendResult.last_sequence_number === 1n, `${storeName} expected last sequence number 1n`);
  assert(appendResult.committed_count === 1n, `${storeName} expected committed count 1n`);

  await waitForCondition(`${storeName} initial streamAll delivery`, () => streamAllBatches.length === 1);
  await waitForCondition(`${storeName} failing callback delivery`, () => thrownBatches.length === 1);
  assert(streamAllBatches[0].length === 1, `${storeName} expected one event in the initial batch`);
  assert(streamAllBatches[0][0].sequence_number === 1n, `${storeName} expected initial sequence number 1n`);

  const filteredQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
      },
    ],
  };
  const filteredSubscription = store.streamTo(filteredQuery, (events) => {
    filteredBatches.push(events);
  });
  const healthySubscription = store.streamTo(filteredQuery, (events) => {
    healthyBatches.push(events);
  });

  const mixedBatchAppendResult = store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 25 },
    },
    {
      event_type: 'account-tagged',
      payload: { accountId, tag: 'vip' },
    },
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 40 },
    },
  ]);
  assert(mixedBatchAppendResult.first_sequence_number === 2n, `${storeName} expected mixed batch first sequence 2n`);
  assert(mixedBatchAppendResult.last_sequence_number === 4n, `${storeName} expected mixed batch last sequence 4n`);
  assert(mixedBatchAppendResult.committed_count === 3n, `${storeName} expected mixed batch committed count 3n`);

  await waitForCondition(`${storeName} filtered delivery`, () => filteredBatches.length === 1);
  await waitForCondition(`${storeName} healthy filtered delivery`, () => healthyBatches.length === 1);
  await waitForCondition(`${storeName} second streamAll delivery`, () => streamAllBatches.length === 2);
  await waitForCondition(`${storeName} second failing callback delivery`, () => thrownBatches.length === 2);

  assert(streamAllBatches[1].length === 3, `${storeName} expected one committed batch with three events`);
  assert(filteredBatches[0].length === 2, `${storeName} expected filtered batch to contain two events`);
  assert(filteredBatches[0][0].event_type === 'money-deposited', `${storeName} expected filtered event type`);
  assert(filteredBatches[0][1].event_type === 'money-deposited', `${storeName} expected filtered event type`);
  assert(healthyBatches[0].length === 2, `${storeName} expected healthy filtered batch to contain two events`);

  const conflictResult: AppendIfResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId, amount: 50 },
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
  assert(conflictResult.append_result == null, `${storeName} expected no append result on conflict`);
  assert(conflictResult.conflict != null, `${storeName} expected explicit conflict result`);
  assert(
    conflictResult.conflict.expected_context_version === 0n,
    `${storeName} expected conflict expected_context_version 0n`,
  );
  assert(
    conflictResult.conflict.actual_context_version === 1n,
    `${storeName} expected conflict actual_context_version 1n`,
  );

  await waitForNoNewDeliveries(`${storeName} streamAll conflict deliveries`, () => streamAllBatches.length, 2);
  await waitForNoNewDeliveries(`${storeName} filtered conflict deliveries`, () => filteredBatches.length, 1);
  await waitForNoNewDeliveries(`${storeName} healthy conflict deliveries`, () => healthyBatches.length, 1);
  await waitForNoNewDeliveries(`${storeName} failing conflict deliveries`, () => thrownBatches.length, 2);

  filteredSubscription.unsubscribe();
  healthySubscription.unsubscribe();
  streamAllSubscription.unsubscribe();
  failingSubscription.unsubscribe();
  filteredSubscription.unsubscribe();
  streamAllSubscription.unsubscribe();

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 75 },
    },
  ]);

  await waitForNoNewDeliveries(`${storeName} streamAll deliveries after unsubscribe`, () => streamAllBatches.length, 2);
  await waitForNoNewDeliveries(`${storeName} filtered deliveries after unsubscribe`, () => filteredBatches.length, 1);
  await waitForNoNewDeliveries(`${storeName} healthy deliveries after unsubscribe`, () => healthyBatches.length, 1);
  await waitForNoNewDeliveries(`${storeName} failing deliveries after unsubscribe`, () => thrownBatches.length, 2);
}
