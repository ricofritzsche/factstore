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
  const failedReplayStream: DurableStream = { name: 'direct-native-durable-rejected-replay' };
  const falseReplayStream: DurableStream = { name: 'direct-native-durable-false-replay' };
  const failingFutureStream: DurableStream = { name: 'direct-native-durable-rejected-future' };
  const falseFutureStream: DurableStream = { name: 'direct-native-durable-false-future' };
  const healthyFutureStream: DurableStream = { name: 'direct-native-durable-healthy-future' };
  const futureQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
      },
    ],
  };
  const failedReplayBatches: EventRecord[][] = [];
  const falseReplayBatches: EventRecord[][] = [];
  const replayedBatches: EventRecord[][] = [];
  const replayedFalseBatches: EventRecord[][] = [];
  const failingFutureBatches: EventRecord[][] = [];
  const falseFutureBatches: EventRecord[][] = [];
  const healthyFutureBatches: EventRecord[][] = [];
  const replayedRejectedFutureBatches: EventRecord[][] = [];
  const replayedFalseFutureBatches: EventRecord[][] = [];

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
    await store.streamAllDurable(failedReplayStream, async (events) => {
      failedReplayBatches.push(events);
      throw new Error('direct native replay callback should reject without advancing the durable cursor');
    });
  } catch (error) {
    failedReplayError = error;
  }

  assert(failedReplayError != null, 'direct native rejected replay should surface a backend failure during registration');
  assert(failedReplayBatches.length === 1, 'direct native rejected replay should still observe the committed batch');
  assert(failedReplayBatches[0][0].sequence_number === 1n, 'direct native rejected replay should observe sequence 1n');

  let falseReplayError: unknown = null;
  try {
    await store.streamAllDurable(falseReplayStream, async (events) => {
      falseReplayBatches.push(events);
      return false;
    });
  } catch (error) {
    falseReplayError = error;
  }

  assert(falseReplayError != null, 'direct native false replay should surface a backend failure during registration');
  assert(falseReplayBatches.length === 1, 'direct native false replay should still observe the committed batch');
  assert(falseReplayBatches[0][0].sequence_number === 1n, 'direct native false replay should observe sequence 1n');

  store = new nativeModule.FactstrSqliteStore(sqliteDatabasePath);
  const replayedSubscription = await store.streamAllDurable(failedReplayStream, (events) => {
    replayedBatches.push(events);
  });
  await waitForCondition('direct native durable replay retry', () => replayedBatches.length === 1);
  assert(replayedBatches[0][0].sequence_number === 1n, 'direct native durable cursor should not advance after rejected replay');
  replayedSubscription.unsubscribe();

  const replayedFalseSubscription = await store.streamAllDurable(falseReplayStream, (events) => {
    replayedFalseBatches.push(events);
  });
  await waitForCondition('direct native durable false replay retry', () => replayedFalseBatches.length === 1);
  assert(replayedFalseBatches[0][0].sequence_number === 1n, 'direct native durable cursor should not advance after false replay');
  replayedFalseSubscription.unsubscribe();

  const failingFutureSubscription = await store.streamToDurable(
    failingFutureStream,
    futureQuery,
    async (events) => {
      failingFutureBatches.push(events);
      throw new Error('direct native future callback should reject without breaking other subscribers');
    },
  );
  const falseFutureSubscription = await store.streamToDurable(
    falseFutureStream,
    futureQuery,
    async (events) => {
      falseFutureBatches.push(events);
      return false;
    },
  );
  const healthyFutureSubscription = await store.streamToDurable(
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

  await waitForCondition('direct native durable rejected future callback', () => failingFutureBatches.length === 1);
  await waitForCondition('direct native durable false future callback', () => falseFutureBatches.length === 1);
  await waitForCondition('direct native durable healthy future callback', () => healthyFutureBatches.length === 1);
  assert(healthyFutureBatches[0][0].sequence_number === 2n, 'direct native healthy durable subscriber should still receive future delivery');

  failingFutureSubscription.unsubscribe();
  falseFutureSubscription.unsubscribe();
  healthyFutureSubscription.unsubscribe();

  const replayedRejectedFutureSubscription = await store.streamToDurable(
    failingFutureStream,
    futureQuery,
    (events) => {
      replayedRejectedFutureBatches.push(events);
    },
  );
  await waitForCondition(
    'direct native durable rejected future replay',
    () => replayedRejectedFutureBatches.length === 1,
  );
  assert(
    replayedRejectedFutureBatches[0][0].sequence_number === 2n,
    'direct native durable cursor should not advance after rejected future delivery',
  );
  replayedRejectedFutureSubscription.unsubscribe();

  const replayedFalseFutureSubscription = await store.streamToDurable(
    falseFutureStream,
    futureQuery,
    (events) => {
      replayedFalseFutureBatches.push(events);
    },
  );
  await waitForCondition(
    'direct native durable false future replay',
    () => replayedFalseFutureBatches.length === 1,
  );
  assert(
    replayedFalseFutureBatches[0][0].sequence_number === 2n,
    'direct native durable cursor should not advance after false future delivery',
  );
  replayedFalseFutureSubscription.unsubscribe();
}
