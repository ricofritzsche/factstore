export interface InteropNewEvent {
  event_type: string;
  payload: unknown;
}

export interface InteropEventFilter {
  event_types?: string[] | null;
  payload_predicates?: unknown[] | null;
}

export interface InteropEventQuery {
  filters?: InteropEventFilter[] | null;
  min_sequence_number?: bigint | null;
}

export interface InteropEventRecord {
  sequence_number: bigint;
  event_type: string;
  payload: unknown;
}

export interface InteropQueryResult {
  event_records: InteropEventRecord[];
  last_returned_sequence_number?: bigint | null;
  current_context_version?: bigint | null;
}

export interface InteropAppendResult {
  first_sequence_number: bigint;
  last_sequence_number: bigint;
  committed_count: bigint;
}

export interface InteropConditionalAppendConflict {
  expected_context_version?: bigint | null;
  actual_context_version?: bigint | null;
}

export interface AppendIfResult {
  append_result?: InteropAppendResult | null;
  conflict?: InteropConditionalAppendConflict | null;
}

export declare class FactstrMemoryStore {
  constructor();
  append(events: InteropNewEvent[]): InteropAppendResult;
  query(query: InteropEventQuery): InteropQueryResult;
  appendIf(
    events: InteropNewEvent[],
    query: InteropEventQuery,
    expectedContextVersion?: bigint | null,
  ): AppendIfResult;
}
