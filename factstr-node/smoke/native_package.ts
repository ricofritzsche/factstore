import {
  type DurableStream,
  type EventQuery,
  type EventRecord,
  type NewEvent,
} from '@factstr/factstr-node';

declare const process: {
  arch: string;
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

type DirectNativeSqliteStore = {
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
  ): {
    append_result?: {
      first_sequence_number: bigint;
      last_sequence_number: bigint;
      committed_count: bigint;
    } | null;
    conflict?: {
      expected_context_version?: bigint | null;
      actual_context_version?: bigint | null;
    } | null;
  };
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

export type NativeFactstrSqliteStoreModule = {
  FactstrSqliteStore: new (databasePath: string) => DirectNativeSqliteStore;
};

function isLinuxGnu(): boolean {
  if (process.platform !== 'linux') {
    return false;
  }

  if (process.report?.getReport) {
    return Boolean(process.report.getReport().header?.glibcVersionRuntime);
  }

  return true;
}

export function currentNativeSupportPackageName(): string {
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

export function loadNativeSqliteStoreModuleDirectly(): NativeFactstrSqliteStoreModule {
  return require(currentNativeSupportPackageName()) as NativeFactstrSqliteStoreModule;
}
