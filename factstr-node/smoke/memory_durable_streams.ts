import {
  type DurableStream,
  type EventQuery,
  type EventRecord,
  FactstrMemoryStore,
} from '@factstr/factstr-node';
import { assert } from './smoke_assert';
import { waitForCondition, waitForNoNewDeliveries } from './wait_for_delivery';

export async function runMemoryDurableStreamsSmoke(): Promise<void> {
  const store = new FactstrMemoryStore();
  const allDurableStream: DurableStream = { name: 'memory-durable-all' };
  const filteredDurableStream: DurableStream = { name: 'memory-durable-filtered' };
  const filteredQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
      },
    ],
  };
  const allBatches: EventRecord[][] = [];
  const filteredBatches: EventRecord[][] = [];

  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-durable-1', owner: 'Rico' },
    },
  ]);
  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'memory-durable-1', amount: 30 },
    },
  ]);

  const allSubscription = await store.streamAllDurable(allDurableStream, (events) => {
    allBatches.push(events);
  });
  const filteredSubscription = await store.streamToDurable(
    filteredDurableStream,
    filteredQuery,
    (events) => {
      filteredBatches.push(events);
    },
  );

  await waitForCondition('memory durable all replay', () => allBatches.length === 2);
  await waitForCondition('memory durable filtered replay', () => filteredBatches.length === 1);
  assert(allBatches[0][0].sequence_number === 1n, 'memory durable replay should preserve sequence 1n');
  assert(allBatches[1][0].sequence_number === 2n, 'memory durable replay should preserve sequence 2n');
  assert(filteredBatches[0].length === 1, 'memory durable filtered replay should deliver one matching event');
  assert(filteredBatches[0][0].sequence_number === 2n, 'memory durable filtered replay should skip non-matching history');

  store.append([
    {
      event_type: 'account-tagged',
      payload: { accountId: 'memory-durable-1', tag: 'vip' },
    },
    {
      event_type: 'money-deposited',
      payload: { accountId: 'memory-durable-1', amount: 15 },
    },
  ]);
  await waitForCondition('memory durable all future delivery', () => allBatches.length === 3);
  await waitForCondition('memory durable filtered future delivery', () => filteredBatches.length === 2);
  assert(allBatches[2].length === 2, 'memory durable all should preserve a committed mixed batch');
  assert(filteredBatches[1].length === 1, 'memory durable filtered future delivery should keep only matching events');
  assert(filteredBatches[1][0].sequence_number === 4n, 'memory durable filtered future delivery should preserve committed order');

  allSubscription.unsubscribe();
  filteredSubscription.unsubscribe();
  allSubscription.unsubscribe();
  filteredSubscription.unsubscribe();

  const resumedAllBatches: EventRecord[][] = [];
  const resumedFilteredBatches: EventRecord[][] = [];
  const resumedAllSubscription = await store.streamAllDurable(allDurableStream, (events) => {
    resumedAllBatches.push(events);
  });
  const resumedFilteredSubscription = await store.streamToDurable(
    filteredDurableStream,
    filteredQuery,
    (events) => {
      resumedFilteredBatches.push(events);
    },
  );

  await waitForNoNewDeliveries('memory durable same-instance replay after unsubscribe', () => resumedAllBatches.length, 0);
  await waitForNoNewDeliveries('memory durable same-instance filtered replay after unsubscribe', () => resumedFilteredBatches.length, 0);

  store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'memory-durable-1', amount: 20 },
    },
  ]);
  await waitForCondition('memory durable same-instance future delivery', () => resumedAllBatches.length === 1);
  await waitForCondition('memory durable same-instance filtered future delivery', () => resumedFilteredBatches.length === 1);
  assert(resumedAllBatches[0][0].sequence_number === 5n, 'memory durable cursor should remain within one store instance');
  assert(resumedFilteredBatches[0][0].sequence_number === 5n, 'memory durable filtered cursor should remain within one store instance');

  resumedAllSubscription.unsubscribe();
  resumedFilteredSubscription.unsubscribe();

  await exerciseAsyncReplayRegistrationSmoke();
  await exerciseAsyncReplayCursorAdvanceSmoke();
  await exerciseAsyncReplayFailureSmoke();
  await exerciseAsyncDurableLiveCursorSmoke();
  await exerciseReplayReentrySmoke();

  const newStore = new FactstrMemoryStore();
  const newStoreBatches: EventRecord[][] = [];
  const newStoreSubscription = await newStore.streamAllDurable(allDurableStream, (events) => {
    newStoreBatches.push(events);
  });
  await waitForNoNewDeliveries('memory durable new store replay', () => newStoreBatches.length, 0);

  newStore.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-durable-2', owner: 'Rico' },
    },
  ]);
  await waitForCondition('memory durable new store future delivery', () => newStoreBatches.length === 1);
  assert(newStoreBatches[0][0].sequence_number === 1n, 'memory durable state should be limited to one store instance');

  newStoreSubscription.unsubscribe();
}

async function exerciseAsyncReplayRegistrationSmoke(): Promise<void> {
  const store = new FactstrMemoryStore();
  const durableStream: DurableStream = { name: 'memory-durable-async-replay-registration' };
  const replayBatches: EventRecord[][] = [];
  const replayGate: { release: (() => void) | null } = { release: null };
  let registrationResolved = false;

  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-async-replay-1', owner: 'Rico' },
    },
  ]);

  const registrationPromise = store.streamAllDurable(durableStream, async (events) => {
    replayBatches.push(events);
    await new Promise<void>((resolve) => {
      replayGate.release = resolve;
    });
  });
  registrationPromise.then(() => {
    registrationResolved = true;
  });

  await waitForCondition(
    'memory durable async replay callback start',
    () => replayBatches.length === 1 && replayGate.release !== null,
  );
  await waitForNoNewDeliveries(
    'memory durable registration promise before replay settles',
    () => (registrationResolved ? 1 : 0),
    0,
  );

  const replayRelease = replayGate.release;
  if (replayRelease == null) {
    throw new Error('memory durable async replay gate should be available before release');
  }
  replayRelease();
  const subscription = await registrationPromise;
  subscription.unsubscribe();
}

async function exerciseAsyncReplayCursorAdvanceSmoke(): Promise<void> {
  const store = new FactstrMemoryStore();
  const durableStream: DurableStream = { name: 'memory-durable-async-replay-success' };
  const replayBatches: EventRecord[][] = [];
  const resumedReplayBatches: EventRecord[][] = [];
  const replayGate: { release: (() => void) | null } = { release: null };

  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-async-replay-success-1', owner: 'Rico' },
    },
  ]);

  const registrationPromise = store.streamAllDurable(durableStream, async (events) => {
    replayBatches.push(events);
    await new Promise<void>((resolve) => {
      replayGate.release = resolve;
    });
  });

  await waitForCondition(
    'memory durable async replay success callback start',
    () => replayBatches.length === 1 && replayGate.release !== null,
  );

  const replayRelease = replayGate.release;
  if (replayRelease == null) {
    throw new Error('memory durable async replay success gate should be available before release');
  }
  replayRelease();
  const subscription = await registrationPromise;

  subscription.unsubscribe();

  const resumedSubscription = await store.streamAllDurable(durableStream, (events) => {
    resumedReplayBatches.push(events);
  });
  await waitForNoNewDeliveries(
    'memory durable async replay success should advance cursor',
    () => resumedReplayBatches.length,
    0,
  );
  resumedSubscription.unsubscribe();
}

async function exerciseAsyncReplayFailureSmoke(): Promise<void> {
  const store = new FactstrMemoryStore();
  const rejectedStream: DurableStream = { name: 'memory-durable-async-replay-rejected' };
  const falseStream: DurableStream = { name: 'memory-durable-async-replay-false' };
  const rejectedReplayBatches: EventRecord[][] = [];
  const falseReplayBatches: EventRecord[][] = [];
  const replayedRejectedBatches: EventRecord[][] = [];
  const replayedFalseBatches: EventRecord[][] = [];

  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-async-replay-2', owner: 'Rico' },
    },
  ]);

  let rejectedReplayError: unknown = null;
  try {
    await store.streamAllDurable(rejectedStream, async (events) => {
      rejectedReplayBatches.push(events);
      throw new Error('memory durable async replay should reject');
    });
  } catch (error) {
    rejectedReplayError = error;
  }
  assert(rejectedReplayError != null, 'memory durable async replay rejection should fail registration');
  assert(rejectedReplayBatches.length === 1, 'memory durable async replay rejection should still observe the replay batch');

  let falseReplayError: unknown = null;
  try {
    await store.streamAllDurable(falseStream, async (events) => {
      falseReplayBatches.push(events);
      return false;
    });
  } catch (error) {
    falseReplayError = error;
  }
  assert(falseReplayError != null, 'memory durable async replay false should fail registration');
  assert(falseReplayBatches.length === 1, 'memory durable async replay false should still observe the replay batch');

  const replayedRejectedSubscription = await store.streamAllDurable(rejectedStream, (events) => {
    replayedRejectedBatches.push(events);
  });
  await waitForCondition(
    'memory durable async replay rejection retry',
    () => replayedRejectedBatches.length === 1,
  );
  assert(
    replayedRejectedBatches[0][0].sequence_number === 1n,
    'memory durable cursor should remain unchanged after async replay rejection',
  );
  replayedRejectedSubscription.unsubscribe();

  const replayedFalseSubscription = await store.streamAllDurable(falseStream, (events) => {
    replayedFalseBatches.push(events);
  });
  await waitForCondition(
    'memory durable async replay false retry',
    () => replayedFalseBatches.length === 1,
  );
  assert(
    replayedFalseBatches[0][0].sequence_number === 1n,
    'memory durable cursor should remain unchanged after async replay false result',
  );
  replayedFalseSubscription.unsubscribe();
}

async function exerciseAsyncDurableLiveCursorSmoke(): Promise<void> {
  const store = new FactstrMemoryStore();
  const durableStream: DurableStream = { name: 'memory-durable-async-live-cursor' };
  const depositQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
      },
    ],
  };
  const initialLiveBatches: EventRecord[][] = [];
  const replayBeforeResolveBatches: EventRecord[][] = [];
  const replayAfterResolveBatches: EventRecord[][] = [];
  const liveGate: { release: (() => void) | null } = { release: null };
  let liveCallbackSettled = false;

  const initialSubscription = await store.streamToDurable(
    durableStream,
    depositQuery,
    async (events) => {
      initialLiveBatches.push(events);
      await new Promise<void>((resolve) => {
        liveGate.release = resolve;
      });
      liveCallbackSettled = true;
    },
  );

  const appendResult = store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId: 'memory-async-live-1', amount: 75 },
    },
  ]);
  assert(appendResult.first_sequence_number === 1n, 'memory durable async live append should still succeed immediately');

  await waitForCondition(
    'memory durable async live callback start',
    () => initialLiveBatches.length === 1 && liveGate.release !== null,
  );

  initialSubscription.unsubscribe();

  const replayBeforeResolveSubscription = await store.streamToDurable(
    durableStream,
    depositQuery,
    (events) => {
      replayBeforeResolveBatches.push(events);
    },
  );
  await waitForCondition(
    'memory durable async live replay before resolve',
    () => replayBeforeResolveBatches.length === 1,
  );
  assert(
    replayBeforeResolveBatches[0][0].sequence_number === 1n,
    'memory durable cursor should not advance before the async live callback resolves',
  );
  replayBeforeResolveSubscription.unsubscribe();

  const liveRelease = liveGate.release;
  if (liveRelease == null) {
    throw new Error('memory durable async live gate should be available before release');
  }
  liveRelease();
  await waitForCondition(
    'memory durable async live callback settled',
    () => liveCallbackSettled,
  );

  const replayAfterResolveSubscription = await store.streamToDurable(
    durableStream,
    depositQuery,
    (events) => {
      replayAfterResolveBatches.push(events);
    },
  );
  await waitForNoNewDeliveries(
    'memory durable async live replay after resolve',
    () => replayAfterResolveBatches.length,
    0,
  );
  replayAfterResolveSubscription.unsubscribe();
}

async function exerciseReplayReentrySmoke(): Promise<void> {
  const store = new FactstrMemoryStore();
  const durableStream: DurableStream = { name: 'memory-durable-replay-reentry' };
  const replayBatches: EventRecord[][] = [];
  let observedCurrentContextVersion: bigint | null = null;

  store.append([
    {
      event_type: 'account-opened',
      payload: { accountId: 'memory-reentry-1', owner: 'Rico' },
    },
  ]);

  const subscription = await store.streamAllDurable(durableStream, async (events) => {
    replayBatches.push(events);
    const result = store.query({
      filters: [
        {
          event_types: ['account-opened'],
          payload_predicates: [{ accountId: 'memory-reentry-1' }],
        },
      ],
    });
    observedCurrentContextVersion = result.current_context_version ?? null;
  });

  await waitForCondition(
    'memory durable replay reentry should complete',
    () => replayBatches.length === 1 && observedCurrentContextVersion === 1n,
  );

  subscription.unsubscribe();
}
