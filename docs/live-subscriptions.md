# Live Subscriptions

FACTSTR currently implements first-class live subscriptions in the shared contract.

## Current Behavior

- subscriptions are live only
- subscriptions do not replay historical events
- `subscribe_all(handle)` observes all future committed batches through one handler
- `subscribe_to(&EventQuery, handle)` observes only future committed facts that match that query
- notifications happen only after a successful commit
- each committed append batch is delivered as one batch
- mixed committed batches are delivered as one filtered batch when matches exist
- delivery order follows committed global sequence order
- failed conditional append emits nothing
- multiple subscribers can observe the same committed batches
- dropping a subscription stops future delivery for that subscriber

This is the projection-oriented use case supported now: a feature slice can subscribe to the facts relevant to its own query model and update that model from the matching committed batches it receives.

## Subscription Methods

- `subscribe_all(handle)` registers a handler for every future committed batch
- `subscribe_to(&EventQuery, handle)` registers a handler filtered by the existing query language
- `HandleEvents` is the callback type invoked with `Vec<EventRecord>` for each delivered batch
- `EventSubscription` is the active registration returned by `subscribe_all(...)` or `subscribe_to(...)`
- `EventSubscription::unsubscribe()` stops future delivery for that subscriber

Each delivered item is a committed batch:

- `Vec<EventRecord>`

This keeps the delivered shape aligned with the append shape instead of splitting one committed append into arbitrary fragments.

For the smallest concrete code path, see the live subscription example in [Examples](examples.md).

## Projection Use Case

A self-contained feature slice can:

- define an `EventQuery` for the facts it cares about
- call `subscribe_to(&EventQuery, handle)` once
- receive only matching future committed facts by contract
- update its own query model inside the handler from each delivered batch

This means the feature slice does not need ad-hoc manual filtering after delivery. Unrelated facts are excluded by the subscription contract itself.

## Current Limitations

The current implementation does not provide:

- durable subscriber cursors
- replay
- snapshots for subscriber catch-up
- transport or pub/sub adapters

This means subscriptions are for future live delivery only. Durable subscriber state is later work.

## Implementation Boundary

The shared contract defines the observable subscription behavior.

Notifier mechanics remain local to each store implementation. The contract does not currently expose a shared notifier abstraction.
