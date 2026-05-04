import {
  type EventQuery,
  type EventRecord,
  FactstrMemoryStore,
  FactstrSqliteStore,
} from '@factstr/factstr-node';
import { assert } from './smoke_assert';
import { createSqliteDatabasePaths } from './sqlite_database_paths';

declare function require(moduleName: string): any;

const { existsSync } = require('node:fs');

type AppendQueryAppendIfStore = Pick<
  FactstrMemoryStore,
  'append' | 'appendIf' | 'query'
>;

export async function runAppendQueryAppendIfSmoke(): Promise<void> {
  await exerciseAppendQueryAppendIfStore('memory', new FactstrMemoryStore(), 'memory-append-1');

  const sqlitePaths = createSqliteDatabasePaths('factstr-node-smoke-append-query-');
  const sqliteDatabasePath = sqlitePaths.databasePath('append-query-append-if');
  assert(!existsSync(sqliteDatabasePath), 'sqlite smoke database should not exist before append');

  await exerciseAppendQueryAppendIfStore(
    'sqlite',
    new FactstrSqliteStore(sqliteDatabasePath),
    'sqlite-append-1',
  );

  assert(existsSync(sqliteDatabasePath), 'sqlite smoke database should exist after append');

  const reopenedStore = new FactstrSqliteStore(sqliteDatabasePath);
  const reopenedQuery = reopenedStore.query({
    filters: [
      {
        payload_predicates: [{ accountId: 'sqlite-append-1' }],
      },
    ],
  });
  assert(reopenedQuery.event_records.length === 4, 'reopened sqlite store should retain appended facts');
  assert(
    reopenedQuery.last_returned_sequence_number === 4n,
    'reopened sqlite store should retain last returned sequence number',
  );
  assert(
    reopenedQuery.current_context_version === 4n,
    'reopened sqlite store should retain current context version',
  );
}

async function exerciseAppendQueryAppendIfStore(
  storeName: string,
  store: AppendQueryAppendIfStore,
  accountId: string,
): Promise<void> {
  const accountOpenedQuery: EventQuery = {
    filters: [
      {
        event_types: ['account-opened'],
        payload_predicates: [{ accountId }],
      },
    ],
  };
  const depositQuery: EventQuery = {
    filters: [
      {
        event_types: ['money-deposited'],
        payload_predicates: [{ accountId }],
      },
    ],
  };
  const accountTagQuery: EventQuery = {
    filters: [
      {
        event_types: ['account-tagged'],
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
  assert(openResult.first_sequence_number === 1n, `${storeName} expected first open sequence 1n`);
  assert(openResult.last_sequence_number === 1n, `${storeName} expected last open sequence 1n`);
  assert(openResult.committed_count === 1n, `${storeName} expected open committed count 1n`);

  const depositResult = store.append([
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 25 },
    },
    {
      event_type: 'money-deposited',
      payload: { accountId, amount: 40 },
    },
  ]);
  assert(depositResult.first_sequence_number === 2n, `${storeName} expected deposit first sequence 2n`);
  assert(depositResult.last_sequence_number === 3n, `${storeName} expected deposit last sequence 3n`);
  assert(depositResult.committed_count === 2n, `${storeName} expected deposit committed count 2n`);

  const accountOpenedResult = store.query(accountOpenedQuery);
  assert(accountOpenedResult.event_records.length === 1, `${storeName} expected one account-opened fact`);
  assert(
    accountOpenedResult.event_records[0].sequence_number === 1n,
    `${storeName} expected account-opened sequence 1n`,
  );
  assert(
    accountOpenedResult.last_returned_sequence_number === 1n,
    `${storeName} expected account-opened last returned sequence 1n`,
  );
  assert(
    accountOpenedResult.current_context_version === 1n,
    `${storeName} expected account-opened current context version 1n`,
  );

  const currentContextVersion = accountOpenedResult.current_context_version ?? null;
  const appendIfSuccessResult = store.appendIf(
    [
      {
        event_type: 'account-tagged',
        payload: { accountId, tag: 'vip' },
      },
    ],
    accountOpenedQuery,
    currentContextVersion,
  );
  assert(
    appendIfSuccessResult.append_result != null,
    `${storeName} expected appendIf to succeed with the current context version`,
  );
  assert(
    appendIfSuccessResult.append_result?.first_sequence_number === 4n,
    `${storeName} expected appendIf success first sequence 4n`,
  );
  assert(
    appendIfSuccessResult.append_result?.last_sequence_number === 4n,
    `${storeName} expected appendIf success last sequence 4n`,
  );
  assert(
    appendIfSuccessResult.append_result?.committed_count === 1n,
    `${storeName} expected appendIf success committed count 1n`,
  );
  assert(appendIfSuccessResult.conflict == null, `${storeName} did not expect appendIf conflict on success`);

  const accountTagResult = store.query(accountTagQuery);
  assert(accountTagResult.event_records.length === 1, `${storeName} expected one account-tagged fact`);
  assert(
    accountTagResult.event_records[0].sequence_number === 4n,
    `${storeName} expected account-tagged sequence 4n`,
  );

  const appendIfConflictResult = store.appendIf(
    [
      {
        event_type: 'money-deposited',
        payload: { accountId, amount: 60 },
      },
    ],
    accountOpenedQuery,
    0n,
  );
  assert(
    appendIfConflictResult.append_result == null,
    `${storeName} expected appendIf conflict to return no append result`,
  );
  assert(
    appendIfConflictResult.conflict != null,
    `${storeName} expected appendIf conflict to stay explicit`,
  );
  assert(
    appendIfConflictResult.conflict?.expected_context_version === 0n,
    `${storeName} expected conflict expected_context_version 0n`,
  );
  assert(
    appendIfConflictResult.conflict?.actual_context_version === 1n,
    `${storeName} expected conflict actual_context_version 1n`,
  );

  const depositsAfterFirstEvent = store.query({
    ...depositQuery,
    min_sequence_number: 1n,
  });
  assert(
    depositsAfterFirstEvent.event_records.length === 2,
    `${storeName} expected two deposits after sequence 1n`,
  );
  assert(
    depositsAfterFirstEvent.event_records[0].sequence_number === 2n,
    `${storeName} expected first returned deposit at sequence 2n`,
  );
  assert(
    depositsAfterFirstEvent.event_records[1].sequence_number === 3n,
    `${storeName} expected second returned deposit at sequence 3n`,
  );
  assert(
    depositsAfterFirstEvent.last_returned_sequence_number === 3n,
    `${storeName} expected last returned sequence 3n after min_sequence_number 1n`,
  );
  assert(
    depositsAfterFirstEvent.current_context_version === 3n,
    `${storeName} expected current context version 3n after min_sequence_number 1n`,
  );

  const depositsAfterSecondDeposit = store.query({
    ...depositQuery,
    min_sequence_number: 2n,
  });
  assert(
    depositsAfterSecondDeposit.event_records.length === 1,
    `${storeName} expected min_sequence_number to be exclusive`,
  );
  assert(
    depositsAfterSecondDeposit.event_records[0].sequence_number === 3n,
    `${storeName} expected only the event after sequence 2n`,
  );

  const depositsAfterAllDeposits = store.query({
    ...depositQuery,
    min_sequence_number: 3n,
  });
  assert(
    depositsAfterAllDeposits.event_records.length === 0,
    `${storeName} expected no deposits after sequence 3n`,
  );
  assert(
    depositsAfterAllDeposits.last_returned_sequence_number == null,
    `${storeName} expected no last returned sequence when no rows are returned`,
  );
  assert(
    depositsAfterAllDeposits.current_context_version === 3n,
    `${storeName} expected current context version to still describe the full matching context`,
  );

  assertOccurredAtStrings(storeName, [
    ...accountOpenedResult.event_records,
    ...depositsAfterFirstEvent.event_records,
    ...accountTagResult.event_records,
  ]);
}

function assertOccurredAtStrings(storeName: string, eventRecords: EventRecord[]): void {
  for (const eventRecord of eventRecords) {
    assert(typeof eventRecord.occurred_at === 'string', `${storeName} expected occurred_at to be a string`);
    assert(eventRecord.occurred_at.length > 0, `${storeName} expected occurred_at to be non-empty`);
  }
}
