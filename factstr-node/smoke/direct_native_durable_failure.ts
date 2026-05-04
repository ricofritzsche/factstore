import { type DurableStream, type EventQuery, type EventRecord } from '@factstr/factstr-node';
import { loadNativeSqliteStoreModuleDirectly } from './native_package';
import { assert } from './smoke_assert';
import { createSqliteDatabasePaths } from './sqlite_database_paths';
import { waitForCondition } from './wait_for_delivery';

export async function runDirectNativeDurableFailureSmoke(): Promise<void> {
  const sqlitePaths = createSqliteDatabasePaths('factstr-node-smoke-direct-native-');
  await exerciseDirectNativeSqliteDurableFailureStore(
    sqlitePaths.databasePath('direct-native-durable-failure'),
  );
}

async function exerciseDirectNativeSqliteDurableFailureStore(
  sqliteDatabasePath: string,
): Promise<void> {
  const nativeModule = loadNativeSqliteStoreModuleDirectly();
  const failedReplayStream: DurableStream = { name: 'direct-native-durable-failed-replay' };
  const failingFutureStream: DurableStream = { name: 'direct-native-durable-failing-future' };
  const healthyFutureStream: DurableStream = { name: 'direct-native-durable-healthy-future' };
  const futureQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
      },
    ],
  };
  const failedReplayBatches: EventRecord[][] = [];
  const replayedBatches: EventRecord[][] = [];
  const failingFutureBatches: EventRecord[][] = [];
  const healthyFutureBatches: EventRecord[][] = [];

  let store = new nativeModule.FactstrSqliteStore(sqliteDatabasePath);
  const initialAppendResult = store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'direct-native-durable-1', owner: 'Rico' },
    },
  ]);
  assert(initialAppendResult.first_sequence_number === 1n, 'direct native initial append should succeed');

  let failedReplayError: unknown = null;
  try {
    store.streamAllDurable(failedReplayStream, (events) => {
      failedReplayBatches.push(events);
      throw new Error('direct native replay callback should fail without advancing the durable cursor');
    });
  } catch (error) {
    failedReplayError = error;
  }

  assert(failedReplayError != null, 'direct native failed replay should surface a backend failure during registration');
  assert(failedReplayBatches.length === 1, 'direct native failed replay should still observe the committed batch');
  assert(failedReplayBatches[0][0].sequence_number === 1n, 'direct native failed replay should observe sequence 1n');

  store = new nativeModule.FactstrSqliteStore(sqliteDatabasePath);
  const replayedSubscription = store.streamAllDurable(failedReplayStream, (events) => {
    replayedBatches.push(events);
  });
  await waitForCondition('direct native durable replay retry', () => replayedBatches.length === 1);
  assert(replayedBatches[0][0].sequence_number === 1n, 'direct native durable cursor should not advance after failed replay');
  replayedSubscription.unsubscribe();

  const failingFutureSubscription = store.streamToDurable(
    failingFutureStream,
    futureQuery,
    (events) => {
      failingFutureBatches.push(events);
      throw new Error('direct native future callback should fail without breaking other subscribers');
    },
  );
  const healthyFutureSubscription = store.streamToDurable(
    healthyFutureStream,
    futureQuery,
    (events) => {
      healthyFutureBatches.push(events);
    },
  );

  const futureAppendResult = store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'direct-native-durable-1', amount: 25 },
    },
  ]);
  assert(futureAppendResult.first_sequence_number === 2n, 'direct native future append should still succeed');
  assert(futureAppendResult.last_sequence_number === 2n, 'direct native future append should still commit exactly one event');

  await waitForCondition('direct native durable failing future callback', () => failingFutureBatches.length === 1);
  await waitForCondition('direct native durable healthy future callback', () => healthyFutureBatches.length === 1);
  assert(healthyFutureBatches[0][0].sequence_number === 2n, 'direct native healthy durable subscriber should still receive future delivery');

  failingFutureSubscription.unsubscribe();
  healthyFutureSubscription.unsubscribe();
}
