# AGENTS.md

## Purpose

This repository builds `factstore`, a Rust event store centered on facts, command-context consistency, and multiple store implementations.

Work in this repository must preserve clear semantics, small local ownership, and stable behavior.

The goal is not to recreate familiar enterprise shapes.  
The goal is to build a durable, understandable system.

## Project Direction

`factstore` is built around these ideas:

- events are immutable facts
- facts live in an append-only log
- consistency is checked against the facts relevant to a command
- one shared semantic contract can have multiple store implementations
- the default shape should work well without requiring external infrastructure
- PostgreSQL remains a first-class store option

## Working Assumption

Do not rely on external repositories, hidden context, or prior implementations that are not present here.

If a behavior matters, it must be visible in this repository through one or more of these:

- public types
- tests
- docs
- explicit task instructions

When in doubt, follow the contract and tests in this repository.

## Non-Negotiable Rules

### 1. Preserve semantics

Unless a task explicitly changes the contract, preserve these meanings:

- event records are append-only
- sequence numbers are global and monotonically increasing
- a committed batch receives one consecutive sequence range
- reads return events in ascending sequence order
- `minSequenceNumber` is a read cursor only
- conditional append checks the full conflict context
- notifications happen only after persistence succeeds
- subscriber failure does not roll back a successful append

### 2. Keep consistency context-based

Do not reshape the project around aggregate-centric defaults.

Do not assume:

- aggregate roots
- stream-per-entity as the primary model
- entity ownership as the default conflict boundary
- rich domain objects as the center of consistency

IDs may appear in facts. They do not define the core model.

### 3. One contract, multiple stores

The shared contract is the product.

Memory, embedded persistent, and PostgreSQL stores may differ internally, but they must preserve the same observable behavior unless the contract explicitly allows a difference.

### 4. Keep important meanings explicit

Do not hide multiple meanings behind one value or one overloaded API.

If the contract distinguishes between returned sequence position and conflict context version, keep that distinction explicit in types, names, tests, and code.

### 5. Keep store mechanics local

Do not leak store-specific implementation details into shared contract types unless they are true cross-store semantics.

Examples that should usually stay local to a store:

- SQL details
- WAL segment layout
- mmap internals
- RocksDB-specific mechanics
- transaction implementation details

## Repository Shape

This repository must stay easy to reason about for humans and coding agents.

Prefer:

- explicit crate purposes
- small modules with visible ownership
- self-contained implementation units
- tests close to the behavior they prove
- names that describe what the code owns

Avoid generic technical buckets and vague module names such as:

- `core`
- `domain`
- `shared`
- `common`
- `utils`
- `helpers`
- `services`
- `managers`
- `repositories`
- `models`
- `entities`

If you add a new module or crate, its name must explain what it owns.

## Naming Rules

Use names that describe semantics directly.

Prefer names like:

- `event_record`
- `new_event`
- `event_query`
- `query_result`
- `append_result`
- `context_version`
- `last_returned_sequence`
- `sequence_number`
- `memory_store`
- `postgres_store`
- `file_store`
- `live_subscription`

Avoid vague names like:

- `service`
- `manager`
- `repository`
- `helper`
- `util`
- `processor`
- `base_model`
- `shared_helpers`

## How To Work In This Repo

### Start from behavior

Before changing code, state in plain language what must be true after the change.

Examples:

- append assigns one consecutive global sequence range
- `minSequenceNumber` affects reads only
- conditional append checks the full conflict context
- notifications happen only after persistence succeeds

Do not start by introducing abstractions.

### Decide whether the change is shared or local

Ask:

- is this part of the shared contract?
- should every store behave this way?
- or is this only one store's mechanism?

If it is shared:
- reflect it in shared types and shared tests

If it is local:
- keep it local to the owning store

### Implement the smallest coherent change

Prefer:

- small local changes
- direct tests of behavior
- visible ownership
- literal names

Avoid:

- speculative extension points
- internal frameworks
- wide cleanup refactors with no semantic gain
- generic wrappers around obvious code

### Keep subscriptions outside the commit path

Notifications happen after persistence succeeds.

Subscriber behavior must never weaken append correctness.

## Testing Rules

Tests are part of the contract.

Prefer tests that directly prove:

- append ordering
- sequence allocation
- batch behavior
- query semantics
- conditional append correctness
- `minSequenceNumber` behavior
- separation of returned sequence and context version
- notification timing
- subscriber isolation from commit success

When possible, keep semantic tests reusable across store implementations.

## New Store Implementations

When adding a store:

1. implement the shared contract first
2. preserve the same observable query and append behavior
3. make concurrency behavior explicit
4. make durability and recovery explicit if the store is persistent
5. reuse semantic tests wherever possible
6. add store-specific tests only where the mechanism truly differs

Do not let a backend redefine the project around its own constraints.

## Module structure inside a crate

`src/lib.rs` is a thin module root.

Do not keep growing implementation in `src/lib.rs` once a crate owns real behavior.

Use `src/lib.rs` only to:

- declare modules
- re-export the public surface
- keep the crate entry point readable

Move implementation into ownership-based files as soon as behavior becomes non-trivial.

Prefer file names that describe owned behavior, for example:

- `memory_store.rs`
- `query.rs`
- `payload_match.rs`
- `conditional_append.rs`

Do not use `lib.rs` as a catch-all implementation file beyond the smallest bootstrap.

## Test placement

Use inline `#[cfg(test)]` tests only for tiny local checks during early bootstrap.

Once a crate owns real behavior, prefer tests in `<crate>/tests/`.

Keep tests separated from production code and organized by behavior.

Reflect the production structure in the test layout when useful.

Examples:

- `factstore-memory/tests/append.rs`
- `factstore-memory/tests/query.rs`
- `factstore-memory/tests/conditional_append.rs`
- `factstore-memory/tests/payload_match.rs`

A growing semantic test suite should not stay embedded in one `src/lib.rs`.

## When more files are justified

Creating more files is justified when it improves:

- ownership clarity
- semantic readability
- test separation
- local reasoning

This is not speculative abstraction.

Do not avoid structure when real behavior already exists.

## What To Avoid

Do not introduce:

- generic service layers
- repository patterns
- manager objects
- shared helper folders
- base modules
- broad “domain” abstractions
- aggregate-centric assumptions
- store-specific leakage into shared contract types

Avoid refactors that mostly rearrange code without improving semantics, ownership, or tests.

## Definition of Done

A task is not done because code compiles.

A task is done when:

- the intended behavior is clear in plain language
- the ownership of the change is obvious
- the names match the meaning
- tests prove the intended behavior
- no speculative abstraction was added
- no generic architecture drift was introduced
- the result still fits the philosophy of facts, context, and stable behavior