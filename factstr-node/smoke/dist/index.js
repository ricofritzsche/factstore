"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const factstr_node_1 = require("factstr-node");
function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
const store = new factstr_node_1.FactstrMemoryStore();
const appendResult = store.append([
    {
        event_type: 'account-opened',
        payload: { accountId: 'a-1', owner: 'Rico' },
    },
]);
assert(appendResult.first_sequence_number === 1n, 'expected first sequence number 1n');
assert(appendResult.last_sequence_number === 1n, 'expected last sequence number 1n');
assert(appendResult.committed_count === 1n, 'expected committed count 1n');
const queryResult = store.query({
    filters: [
        {
            event_types: ['account-opened'],
            payload_predicates: [{ accountId: 'a-1' }],
        },
    ],
});
assert(queryResult.event_records.length === 1, 'expected one queried event');
assert(queryResult.event_records[0].sequence_number === 1n, 'expected queried sequence number 1n');
assert(queryResult.last_returned_sequence_number === 1n, 'expected last returned sequence number 1n');
assert(queryResult.current_context_version === 1n, 'expected current context version 1n');
const conflictResult = store.appendIf([
    {
        event_type: 'money-deposited',
        payload: { accountId: 'a-1', amount: 25 },
    },
], {
    filters: [
        {
            event_types: ['account-opened'],
        },
    ],
}, 0n);
assert(conflictResult.append_result == null, 'expected no append result on conflict');
assert(conflictResult.conflict != null, 'expected explicit conflict result');
assert(conflictResult.conflict.expected_context_version === 0n, 'expected conflict expected_context_version 0n');
assert(conflictResult.conflict.actual_context_version === 1n, 'expected conflict actual_context_version 1n');
console.log('factstr-node TypeScript smoke test passed');
