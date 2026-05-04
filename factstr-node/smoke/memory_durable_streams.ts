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

  const allSubscription = store.streamAllDurable(allDurableStream, (events) => {
    allBatches.push(events);
  });
  const filteredSubscription = store.streamToDurable(
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
  const resumedAllSubscription = store.streamAllDurable(allDurableStream, (events) => {
    resumedAllBatches.push(events);
  });
  const resumedFilteredSubscription = store.streamToDurable(
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

  const newStore = new FactstrMemoryStore();
  const newStoreBatches: EventRecord[][] = [];
  const newStoreSubscription = newStore.streamAllDurable(allDurableStream, (events) => {
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
