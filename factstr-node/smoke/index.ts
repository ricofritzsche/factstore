import {
  type AppendIfResult,
  type DurableStream,
  type EventQuery,
  type NewEvent,
  type EventRecord,
  FactstrMemoryStore,
  FactstrSqliteStore,
} from '@factstr/factstr-node';

declare const process: {
  arch: string;
  exitCode?: number;
  on(event: string, listener: () => void): void;
  platform: string;
  report?: {
    getReport?: () => {
      header?: {
        glibcVersionRuntime?: string;
      };
    };
  };
};
declare function require(moduleName: string): any;

const { mkdtempSync, rmSync } = require('node:fs');
const { join } = require('node:path');
const { tmpdir } = require('node:os');

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

async function waitForCondition(
  description: string,
  condition: () => boolean,
  timeoutMs = 500,
  pollIntervalMs = 10,
): Promise<void> {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (condition()) {
      return;
    }
    await wait(pollIntervalMs);
  }

  throw new Error(`timed out waiting for ${description}`);
}

async function waitForNoNewDeliveries(
  description: string,
  currentCount: () => number,
  expectedCount: number,
  stableMs = 100,
  timeoutMs = 400,
  pollIntervalMs = 10,
): Promise<void> {
  const startedAt = Date.now();
  let stableStartedAt: number | null = null;

  while (Date.now() - startedAt < timeoutMs) {
    const count = currentCount();
    if (count !== expectedCount) {
      throw new Error(
        `${description} changed unexpectedly: expected ${expectedCount}, got ${count}`,
      );
    }

    if (stableStartedAt === null) {
      stableStartedAt = Date.now();
    }

    if (Date.now() - stableStartedAt >= stableMs) {
      return;
    }

    await wait(pollIntervalMs);
  }

  throw new Error(`timed out waiting for stable ${description}`);
}

type StreamCapableStore = {
  append(events: NewEvent[]): {
    first_sequence_number: bigint;
    last_sequence_number: bigint;
    committed_count: bigint;
  };
  query(query: EventQuery): {
    event_records: EventRecord[];
    last_returned_sequence_number?: bigint | null;
    current_context_version?: bigint | null;
  };
  appendIf(
    events: NewEvent[],
    query: EventQuery,
    expectedContextVersion?: bigint | null,
  ): AppendIfResult;
  streamAll(handle: (events: EventRecord[]) => void): { unsubscribe(): void };
  streamTo(
    query: EventQuery,
    handle: (events: EventRecord[]) => void,
  ): { unsubscribe(): void };
  streamAllDurable(
    durableStream: DurableStream,
    handle: (events: EventRecord[]) => void,
  ): { unsubscribe(): void };
  streamToDurable(
    durableStream: DurableStream,
    query: EventQuery,
    handle: (events: EventRecord[]) => void,
  ): { unsubscribe(): void };
};

type NativeFactstrSqliteStoreModule = {
  FactstrSqliteStore: new (databasePath: string) => StreamCapableStore;
};

function currentNativeSupportPackageName(): string {
  if (process.platform === 'darwin' && process.arch === 'arm64') {
    return '@factstr/factstr-node-darwin-arm64';
  }

  if (process.platform === 'darwin' && process.arch === 'x64') {
    return '@factstr/factstr-node-darwin-x64';
  }

  if (
    process.platform === 'linux' &&
    process.arch === 'x64' &&
    isLinuxGnu()
  ) {
    return '@factstr/factstr-node-linux-x64-gnu';
  }

  if (process.platform === 'win32' && process.arch === 'x64') {
    return '@factstr/factstr-node-win32-x64-msvc';
  }

  throw new Error(
    `factstr-node smoke test does not have a direct native package for ${process.platform}-${process.arch}`,
  );
}

function isLinuxGnu(): boolean {
  if (process.platform !== 'linux') {
    return false;
  }

  if (process.report?.getReport) {
    return Boolean(process.report.getReport().header?.glibcVersionRuntime);
  }

  return true;
}

function loadNativeSqliteStoreModuleDirectly(): NativeFactstrSqliteStoreModule {
  return require(currentNativeSupportPackageName()) as NativeFactstrSqliteStoreModule;
}

async function exerciseLiveStore(
  storeName: string,
  store: StreamCapableStore,
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

  const openedAccount: NewEvent = {
    event_type: 'account-opened',
    payload: { accountId, owner: 'Rico' },
  };
  const appendResult = store.append([openedAccount]);

  assert(appendResult.first_sequence_number === 1n, `${storeName} expected first sequence number 1n`);
  assert(appendResult.last_sequence_number === 1n, `${storeName} expected last sequence number 1n`);
  assert(appendResult.committed_count === 1n, `${storeName} expected committed count 1n`);

  await waitForCondition(`${storeName} initial streamAll delivery`, () => streamAllBatches.length === 1);
  await waitForCondition(`${storeName} failing callback delivery`, () => thrownBatches.length === 1);
  assert(streamAllBatches[0].length === 1, `${storeName} expected one event in first batch`);
  assert(streamAllBatches[0][0].sequence_number === 1n, `${storeName} expected first batch sequence 1n`);

  const accountQuery: EventQuery = {
    filters: [
      {
        event_types: ['account-opened'],
        payload_predicates: [{ accountId }],
      },
    ],
  };
  const queryResult = store.query(accountQuery);
  assert(queryResult.event_records.length === 1, `${storeName} expected one queried event`);
  assert(queryResult.event_records[0].sequence_number === 1n, `${storeName} expected queried sequence number 1n`);
  assert(typeof queryResult.event_records[0].occurred_at === 'string', `${storeName} expected occurred_at string`);
  assert(queryResult.event_records[0].occurred_at.length > 0, `${storeName} expected non-empty occurred_at`);
  assert(queryResult.last_returned_sequence_number === 1n, `${storeName} expected last returned sequence number 1n`);
  assert(queryResult.current_context_version === 1n, `${storeName} expected current context version 1n`);

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

  await waitForCondition(`${storeName} filtered stream delivery`, () => filteredBatches.length === 1);
  await waitForCondition(`${storeName} healthy filtered delivery`, () => healthyBatches.length === 1);
  await waitForCondition(`${storeName} second streamAll delivery`, () => streamAllBatches.length === 2);
  await waitForCondition(`${storeName} second failing callback delivery`, () => thrownBatches.length === 2);

  assert(streamAllBatches[1].length === 3, `${storeName} expected one committed batch with three events`);
  assert(filteredBatches[0].length === 2, `${storeName} expected filtered batch to contain two events`);
  assert(filteredBatches[0][0].event_type === 'money-deposited', `${storeName} expected first filtered event type`);
  assert(filteredBatches[0][1].event_type === 'money-deposited', `${storeName} expected second filtered event type`);
  assert(healthyBatches[0].length === 2, `${storeName} expected healthy filtered batch to contain two events`);

  const conflictResult: AppendIfResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId, amount: 50 },
      },
    ],
    accountQuery,
    0n,
  );
  assert(conflictResult.append_result == null, `${storeName} expected no append result on conflict`);
  assert(conflictResult.conflict != null, `${storeName} expected explicit conflict result`);
  assert(conflictResult.conflict.expected_context_version === 0n, `${storeName} expected conflict expected_context_version 0n`);
  assert(conflictResult.conflict.actual_context_version === 1n, `${storeName} expected conflict actual_context_version 1n`);

  await waitForNoNewDeliveries(`${storeName} streamAll conflict deliveries`, () => streamAllBatches.length, 2);
  await waitForNoNewDeliveries(`${storeName} filtered conflict deliveries`, () => filteredBatches.length, 1);
  await waitForNoNewDeliveries(`${storeName} healthy conflict deliveries`, () => healthyBatches.length, 1);
  await waitForNoNewDeliveries(`${storeName} failing callback conflict deliveries`, () => thrownBatches.length, 2);

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

async function exerciseMemoryDurableStore(): Promise<void> {
  const store = new FactstrMemoryStore();
  const durableStream: DurableStream = { name: 'memory-durable-all' };
  const durableBatches: EventRecord[][] = [];
  const accountQuery: EventQuery = {
    filters: [
      {
        event_types: ['account-opened'],
      },
    ],
  };

  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-durable-1', owner: 'Rico' },
    },
  ]);

  const durableSubscription = store.streamAllDurable(durableStream, (events) => {
    durableBatches.push(events);
  });

  await waitForCondition('memory durable replay delivery', () => durableBatches.length === 1);
  assert(durableBatches[0].length === 1, 'memory durable replay should deliver one event');
  assert(durableBatches[0][0].sequence_number === 1n, 'memory durable replay should start at sequence 1n');

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'memory-durable-1', amount: 30 },
    },
  ]);

  await waitForCondition('memory durable future delivery', () => durableBatches.length === 2);
  assert(durableBatches[1][0].sequence_number === 2n, 'memory durable live delivery should continue with sequence 2n');

  const conflictResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId: 'memory-durable-1', amount: 5 },
      },
    ],
    accountQuery,
    0n,
  );
  assert(conflictResult.append_result == null, 'memory durable appendIf conflict should return no append result');
  assert(conflictResult.conflict != null, 'memory durable appendIf conflict should remain explicit');
  await waitForNoNewDeliveries('memory durable conflict deliveries', () => durableBatches.length, 2);

  durableSubscription.unsubscribe();
  durableSubscription.unsubscribe();

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'memory-durable-1', amount: 50 },
    },
  ]);

  await waitForNoNewDeliveries('memory durable deliveries after unsubscribe', () => durableBatches.length, 2);
}

async function exerciseSqliteDurableReplayStore(sqliteDatabasePath: string): Promise<void> {
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

  await waitForCondition('sqlite durable replay deliveries', () => initialBatches.length === 2);
  assert(initialBatches[0].length === 1, 'sqlite durable replay should preserve the first committed batch');
  assert(initialBatches[0][0].sequence_number === 1n, 'sqlite durable replay first sequence should be 1n');
  assert(initialBatches[1].length === 1, 'sqlite durable replay should preserve the second committed batch');
  assert(initialBatches[1][0].sequence_number === 2n, 'sqlite durable replay second sequence should be 2n');

  store.append([
    {
      event_type: 'account-tagged',
      payload: { accountId: 'sqlite-durable-1', tag: 'vip' },
    },
  ]);
  await waitForCondition('sqlite durable future delivery', () => initialBatches.length === 3);
  assert(initialBatches[2][0].sequence_number === 3n, 'sqlite durable future sequence should be 3n');

  const accountQuery: EventQuery = {
    filters: [
      {
        event_types: ['account-opened'],
        payload_predicates: [{ accountId: 'sqlite-durable-1' }],
      },
    ],
  };
  const conflictResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId: 'sqlite-durable-1', amount: 50 },
      },
    ],
    accountQuery,
    0n,
  );
  assert(conflictResult.append_result == null, 'sqlite durable appendIf conflict should return no append result');
  assert(conflictResult.conflict != null, 'sqlite durable appendIf conflict should remain explicit');
  await waitForNoNewDeliveries('sqlite durable conflict deliveries', () => initialBatches.length, 3);

  durableSubscription.unsubscribe();
  durableSubscription.unsubscribe();

  store = new FactstrSqliteStore(sqliteDatabasePath);
  const resumedBatches: EventRecord[][] = [];
  const resumedSubscription = store.streamAllDurable(durableStream, (events) => {
    resumedBatches.push(events);
  });

  await waitForNoNewDeliveries('sqlite durable replay after reopen', () => resumedBatches.length, 0);

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'sqlite-durable-1', amount: 40 },
    },
  ]);
  await waitForCondition('sqlite durable resumed future delivery', () => resumedBatches.length === 1);
  assert(resumedBatches[0][0].sequence_number === 4n, 'sqlite durable replay should resume after the stored cursor');

  resumedSubscription.unsubscribe();
}

async function exerciseSqliteStreamToDurableStore(sqliteDatabasePath: string): Promise<void> {
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
  assert(filteredBatches[0][0].event_type === 'money-deposited', 'sqlite durable filtered replay should only deliver matching events');

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

async function exerciseSqliteDurableFailureIsolationStore(sqliteDatabasePath: string): Promise<void> {
  const failingDurableStream: DurableStream = { name: 'sqlite-durable-failing' };
  const healthyDurableStream: DurableStream = { name: 'sqlite-durable-healthy' };
  const failingBatches: EventRecord[][] = [];
  const healthyBatches: EventRecord[][] = [];

  let store = new FactstrSqliteStore(sqliteDatabasePath);
  const failingSubscription = store.streamAllDurable(failingDurableStream, (events) => {
    failingBatches.push(events);
    throw new Error('sqlite durable failing callback should not break append');
  });
  const healthySubscription = store.streamAllDurable(healthyDurableStream, (events) => {
    healthyBatches.push(events);
  });

  const appendResult = store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'sqlite-durable-failure-1', owner: 'Rico' },
    },
  ]);
  assert(appendResult.first_sequence_number === 1n, 'sqlite durable failure append should still succeed');

  await waitForCondition('sqlite durable failing callback delivery', () => failingBatches.length === 1);
  await waitForCondition('sqlite durable healthy callback delivery', () => healthyBatches.length === 1);
  assert(healthyBatches[0][0].sequence_number === 1n, 'sqlite durable healthy subscriber should still receive the committed batch');

  failingSubscription.unsubscribe();
  healthySubscription.unsubscribe();

  store = new FactstrSqliteStore(sqliteDatabasePath);
  const retriedBatches: EventRecord[][] = [];
  const retrySubscription = store.streamAllDurable(failingDurableStream, (events) => {
    retriedBatches.push(events);
  });

  await waitForCondition('sqlite durable failed cursor replay retry', () => retriedBatches.length === 1);
  assert(retriedBatches[0][0].sequence_number === 1n, 'sqlite durable failure should not advance the cursor past the failed delivery');

  retrySubscription.unsubscribe();
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
  assert(failedReplayBatches[0][0].sequence_number === 1n, 'direct native failed replay should observe the committed batch');

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

const sqliteDirectory = mkdtempSync(join(tmpdir(), 'factstr-node-smoke-'));
const sqliteLiveDatabasePath = join(sqliteDirectory, 'factstr-live.sqlite');
const sqliteDurableReplayDatabasePath = join(sqliteDirectory, 'factstr-durable-replay.sqlite');
const sqliteDurableFilteredDatabasePath = join(sqliteDirectory, 'factstr-durable-filtered.sqlite');
const sqliteDurableFailureDatabasePath = join(sqliteDirectory, 'factstr-durable-failure.sqlite');
const sqliteDirectNativeDurableFailureDatabasePath = join(
  sqliteDirectory,
  'factstr-direct-native-durable-failure.sqlite',
);

process.on('exit', () => {
  rmSync(sqliteDirectory, { recursive: true, force: true });
});

async function main(): Promise<void> {
  await exerciseLiveStore('memory', new FactstrMemoryStore(), 'memory-a-1');
  await exerciseLiveStore('sqlite', new FactstrSqliteStore(sqliteLiveDatabasePath), 'sqlite-a-1');
  await exerciseMemoryDurableStore();
  await exerciseSqliteDurableReplayStore(sqliteDurableReplayDatabasePath);
  await exerciseSqliteStreamToDurableStore(sqliteDurableFilteredDatabasePath);
  await exerciseSqliteDurableFailureIsolationStore(sqliteDurableFailureDatabasePath);
  await exerciseDirectNativeSqliteDurableFailureStore(sqliteDirectNativeDurableFailureDatabasePath);
  console.log('factstr-node TypeScript smoke test passed');
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
