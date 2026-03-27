# AGENTS.md

## Purpose

This repository builds a Rust event store around **facts, command-context consistency, and multiple store implementations**.

The project preserves the semantic contract of the existing [TypeScript eventstore](https://github.com/ricofritzsche/eventstore-typescript) while moving to a more stable and explicit Rust implementation.

The goal is **not** to port TypeScript line by line.

## Core Philosophy

### Facts first

Events are immutable facts in one append-only log.

### Context defines consistency

Consistency is enforced against the facts relevant to a command.
Conflict boundaries are defined by queries, not by aggregate ownership.

### One contract, multiple stores

The product is the semantic contract.
Memory, file-based, and PostgreSQL stores may differ internally, but they must preserve the same observable behavior.

### Explicit semantics

Do not hide important meanings behind overloaded APIs or vague types.
In particular, keep read cursors and conflict context versions explicit.

### Small durable core

Prefer a strong single-node design over speculative distributed design.

## Non-Negotiable Rules

### Preserve the shared semantics

Across store implementations, preserve these semantics unless the user explicitly changes the contract:

* append-only immutable event records
* global monotonically increasing sequence numbers
* consecutive sequence numbers within a committed batch
* query results ordered by ascending sequence number
* `minSequenceNumber` as incremental read cursor only
* context-scoped optimistic locking
* notifications only after successful persistence
* subscriber failure never rolls back a successful append

### Do not reintroduce aggregate-centric defaults

Do not reshape this project into:

* aggregate-root modeling
* stream-per-entity assumptions
* entity ownership as the default conflict boundary
* command handlers built around rich domain object graphs

IDs may exist in facts. They do not define the core consistency model.

### Keep contracts first

When changing behavior, start from the contract and tests.

Prefer to make semantics explicit in:

* public types
* error types
* query results
* append results
* tests

Do not start with internal abstractions and hope the contract emerges later.

## Repository Shape

This repository should be easy for humans and coding agents to reason about locally.

Prefer:

* explicit crate purposes
* precise names
* self-contained implementation units
* small modules with visible ownership
* tests near the behavior they prove

Avoid generic technical buckets and vague folders such as:

* `core`
* `domain`
* `shared`
* `common`
* `utils`
* `helpers`
* `services`
* `managers`
* `repositories`
* `models`
* `entities`

If a new module or crate is introduced, its name must explain **what it owns**, not what technical role it plays.

## Naming Rules

Use names that reflect semantics, not architecture fashion.

Prefer names like:

* `query`
* `append`
* `subscription`
* `context_version`
* `sequence_number`
* `event_record`
* `memory_store`
* `postgres_store`
* `file_store`

Avoid vague names like:

* `engine_utils`
* `store_manager`
* `event_service`
* `base_model`
* `shared_helpers`

## Implementation Rules

### 1. Separate semantic contract from store mechanics

Shared types and traits should define behavior, not internal storage details.

Do not leak:

* SQL assumptions
* WAL segment details
* mmap internals
* RocksDB-specific shapes

into the shared contract unless they are truly cross-store semantics.

### 2. Keep hot-path data simple

Do not force JSON-centric dynamic structures into every hot path if the engine does not need them internally.

### 3. Prefer explicit concurrency models

If a store depends on a concurrency rule, make that rule visible in the implementation and tests.

Examples:

* single-writer sequencing
* post-commit publication
* snapshot-based reads
* transactional conditional append

### 4. Keep subscriptions outside the commit path

Notifications happen after persistence succeeds.
Subscriber behavior must not weaken append correctness.

### 5. Avoid speculative abstractions

Do not add traits, extension points, generic layers, or plugin systems unless there is a concrete implementation need in this repository now.

## Store Implementation Rules

Every store implementation must be judged by the same questions:

* does it preserve the semantic contract?
* does it expose the same read and append behavior?
* does it make context-scoped optimistic locking explicit and correct?
* does it keep post-commit notifications outside the write path?
* does it have clear operational behavior for its own storage model?

It is acceptable for stores to differ in:

* internal indexing strategy
* persistence mechanism
* concurrency implementation
* recovery model
* performance profile
* subscription transport details

It is not acceptable for stores to drift semantically without that being made explicit in the contract and tests.

## Testing Rules

Tests are part of the contract.

Prefer tests that prove:

* append ordering
* sequence allocation
* batch behavior
* query semantics
* conditional append correctness
* `minSequenceNumber` behavior
* separation of `last_returned_sequence` and `context_version`
* notification timing
* subscriber isolation from commit success

When possible, the same semantic test shape should be reusable across multiple store implementations.

## Change Rules

When implementing a change:

1. identify the semantic behavior involved
2. identify whether it is shared contract or store-specific behavior
3. update tests first or alongside the change
4. implement the smallest coherent change
5. verify that naming and structure still reflect local ownership
6. verify that no generic technical bucket was introduced

## Definition of Done

A task is not done because code compiles.

A task is done when:

* the contract is still clear
* the implementation is locally understandable
* tests prove the intended behavior
* no speculative abstraction was added
* no generic architecture drift was introduced
* the result matches the project philosophy in this file
