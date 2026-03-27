---
name: factstore-implementation
description: Implement or change factstore behavior while preserving the shared semantic contract across store implementations.
---

# Purpose

Use this skill when working on `factstore`.

This repository is built around:

- immutable facts
- command-context consistency
- one shared semantic contract
- multiple store implementations

The task is never just to make code compile.

The task is to preserve or improve the semantic contract while keeping the implementation small, explicit, and easy to reason about.

# Use This Skill When

Use this skill for tasks such as:

- implementing or changing the shared contract
- implementing or changing a store
- adding semantic tests
- fixing append, query, sequencing, or subscription behavior
- adding a new store implementation
- tightening names, ownership, or structure around existing behavior

Do not use this skill for generic repository setup, CI, packaging, or unrelated infrastructure work unless the task directly affects factstore behavior.

# Core Project Ideas

## Facts first

Events are immutable facts in one append-only log.

## Context defines consistency

A command checks the version of the facts relevant to its decision context and appends only if that context has not changed.

## One contract, multiple stores

Memory, embedded persistent, and PostgreSQL stores may differ internally, but they must expose the same observable semantics.

## Explicit semantics

Do not overload one value with multiple meanings.

In particular, keep these meanings distinct when the contract requires it:

- the last returned sequence from a read
- the current version of the full conflict context

## Small durable core

Prefer the smallest coherent design that makes the semantics explicit and testable.

# Non-Negotiable Rules

## Preserve shared semantics

Unless the task explicitly changes the contract, preserve these meanings:

- event records are append-only
- sequence numbers are global and monotonically increasing
- a committed batch receives one consecutive sequence range
- query results are ordered by ascending sequence number
- `minSequenceNumber` is a read cursor only
- conditional append checks the full conflict context
- notifications happen only after successful persistence
- subscriber failure does not roll back a successful append

## Do not drift into aggregate-centric defaults

Do not reshape the project into:

- aggregate-root modeling
- stream-per-entity assumptions as the default model
- aggregate ownership as the default conflict boundary
- rich domain object graphs as the center of consistency

IDs may appear in events. They do not define the core model.

## Keep storage details local

Do not leak store-specific mechanics into shared contract types unless they are true cross-store semantics.

Examples of store-specific mechanics that should usually stay local:

- SQL details
- WAL segment layout
- mmap internals
- RocksDB-specific types
- transaction implementation details

## Avoid generic technical buckets

Do not introduce generic folders, files, modules, or types such as:

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

If code is added, its name must explain what it owns.

# Working Method

## 1. Classify the task first

Before changing code, classify the task into one of these:

- shared contract
- memory store
- embedded persistent store
- postgres store
- subscriptions
- tests only
- docs only

If the task touches more than one of these, keep the boundary explicit.

## 2. Restate the intended behavior in plain language

Write down what must be true after the change.

Examples:

- append assigns one consecutive global sequence range
- `minSequenceNumber` affects reads only
- conditional append checks the full conflict context
- notifications happen only after persistence succeeds

Do this before designing new abstractions.

## 3. Decide whether the change is shared or store-specific

Ask:

- is this part of the public contract?
- should every store behave this way?
- or is this only a mechanism of one store?

If the behavior is shared:
- reflect it in shared types and shared tests

If the behavior is store-specific:
- keep it local to that store

## 4. Inspect the nearest existing code and tests

Before adding new structure:

- look for the current contract types
- look for the nearest relevant tests
- look for the store that already owns similar behavior
- look for the smallest place where the change naturally belongs

Do not create new top-level structure unless the current structure clearly cannot own the change.

## 5. Implement the smallest coherent change

Prefer:

- small local modules
- explicit names
- visible ownership
- direct tests of behavior

Avoid:

- speculative extension points
- cleanup refactors with no semantic gain
- generic wrappers around obvious logic
- broad internal frameworks

## 6. Prove the behavior with tests

Tests are part of the contract.

Prefer tests that prove:

- append ordering
- sequence allocation
- batch behavior
- query semantics
- conditional append correctness
- `minSequenceNumber` behavior
- separation of returned sequence and context version
- notification timing
- subscriber isolation from commit success

When possible, keep the same semantic test shape reusable across multiple store implementations.

## 7. Check for architecture drift before finishing

Before considering the task done, verify:

- the names match the meaning
- the ownership of the code is obvious
- the change stayed in the smallest sensible place
- no generic bucket was introduced
- no aggregate-centric assumption slipped in
- tests prove the intended behavior directly

# Implementation Guidance

## Shared contract changes

Use this path when the change affects repository-wide semantics.

Steps:

1. define the behavior in plain language
2. update contract-level types
3. update or add semantic tests
4. update each affected store
5. verify cross-store alignment
6. update docs if the public meaning changed

## Single store changes

Use this path when the change affects one store only.

Steps:

1. identify the shared semantic that must remain true
2. identify the store-specific mechanism involved
3. update tests for that store
4. make the smallest coherent implementation change
5. verify that the store still matches the shared contract

## New store implementations

Use this path when adding a backend.

Steps:

1. implement the shared contract first
2. preserve the same observable query and append behavior
3. make the concurrency model explicit
4. make durability and recovery explicit if the store is persistent
5. reuse semantic tests wherever possible
6. add store-specific tests only where the mechanism differs

Do not let a new backend redefine the project around its own constraints.

# Naming Rules

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


# What To Avoid

Do not introduce:

- generic service layers
- repository patterns
- manager objects
- shared helper folders
- base classes or base modules
- broad “domain” abstractions
- aggregate-centric assumptions
- stream ownership as the default model
- store-specific leakage into shared contract types

Avoid refactors that mainly rearrange code without improving semantics, ownership, or tests.

# Definition of Done

A task is not done because the code compiles.

A task is done when:

- the behavior is clear in plain language
- the ownership of the change is obvious
- the names match the meaning
- the tests prove the intended behavior
- no speculative abstraction was added
- no generic architecture drift was introduced
- the result still fits the project philosophy of facts, context, and stable behavior

# Structural rule for crate internals

Use `src/lib.rs` as a thin module root only.

Once a crate contains real behavior, move implementation out of `src/lib.rs` into ownership-based modules.

`src/lib.rs` should mainly:

- declare modules
- re-export public types
- present the crate surface clearly

Do not let `src/lib.rs` become the main implementation file after bootstrap.

Prefer module names that describe owned behavior directly, for example:

- `memory_store.rs`
- `query.rs`
- `payload_match.rs`
- `conditional_append.rs`

Avoid vague names and avoid one giant internal file.

# Structural rule for tests

Use inline tests only for very small bootstrap checks.

When behavior becomes real and semantic, place tests in `<crate>/tests/`.

Organize test files by behavior and let the test layout reflect the owned behavior of the crate.

Examples:

- `tests/append.rs`
- `tests/query.rs`
- `tests/conditional_append.rs`
- `tests/payload_match.rs`

Do not keep a growing semantic test suite inside one production source file.

# Decision rule for structure

Do not confuse better structure with speculative abstraction.

Splitting code into ownership-based modules and moving tests into a dedicated `tests/` folder is required when it improves:

- semantic clarity
- local reasoning
- maintainability
- auditability

As soon as a crate owns meaningful behavior, prefer clear structure over extreme file minimization.