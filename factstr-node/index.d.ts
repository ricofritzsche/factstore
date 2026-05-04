export interface NewEvent {
  event_type: string;
  payload: unknown;
}

export interface EventFilter {
  event_types?: string[] | null;
  payload_predicates?: unknown[] | null;
}

export interface EventQuery {
  filters?: EventFilter[] | null;
  min_sequence_number?: bigint | null;
}

export interface EventRecord {
  sequence_number: bigint;
  occurred_at: string;
  event_type: string;
  payload: unknown;
}

export interface QueryResult {
  event_records: EventRecord[];
  last_returned_sequence_number?: bigint | null;
  current_context_version?: bigint | null;
}

export interface AppendResult {
  first_sequence_number: bigint;
  last_sequence_number: bigint;
  committed_count: bigint;
}

export interface ConditionalAppendConflict {
  expected_context_version?: bigint | null;
  actual_context_version?: bigint | null;
}

export interface AppendIfResult {
  append_result?: AppendResult | null;
  conflict?: ConditionalAppendConflict | null;
}

export interface DurableStream {
  name: string;
}

export declare class EventStreamSubscription {
  unsubscribe(): void;
}

export declare class FactstrMemoryStore {
  constructor();
  append(events: NewEvent[]): AppendResult;
  query(query: EventQuery): QueryResult;
  appendIf(
    events: NewEvent[],
    query: EventQuery,
    expectedContextVersion?: bigint | null,
  ): AppendIfResult;
  streamAll(handle: (events: EventRecord[]) => void): EventStreamSubscription;
  streamTo(
    query: EventQuery,
    handle: (events: EventRecord[]) => void,
  ): EventStreamSubscription;
  streamAllDurable(
    durableStream: DurableStream,
    handle: (events: EventRecord[]) => void,
  ): EventStreamSubscription;
  streamToDurable(
    durableStream: DurableStream,
    query: EventQuery,
    handle: (events: EventRecord[]) => void,
  ): EventStreamSubscription;
}

export declare class FactstrSqliteStore {
  constructor(databasePath: string);
  append(events: NewEvent[]): AppendResult;
  query(query: EventQuery): QueryResult;
  appendIf(
    events: NewEvent[],
    query: EventQuery,
    expectedContextVersion?: bigint | null,
  ): AppendIfResult;
  streamAll(handle: (events: EventRecord[]) => void): EventStreamSubscription;
  streamTo(
    query: EventQuery,
    handle: (events: EventRecord[]) => void,
  ): EventStreamSubscription;
  streamAllDurable(
    durableStream: DurableStream,
    handle: (events: EventRecord[]) => void,
  ): EventStreamSubscription;
  streamToDurable(
    durableStream: DurableStream,
    query: EventQuery,
    handle: (events: EventRecord[]) => void,
  ): EventStreamSubscription;
}
