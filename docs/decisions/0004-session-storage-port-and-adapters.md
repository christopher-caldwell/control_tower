---
id: ADR-0004
title: Keep session storage behind a core-owned port
type: decision
status: accepted
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: explicit-user-direction-and-established-rust-playbook
decision_date: '2026-09-30'
sources:
- ../history/2026-09-30-initial-design.md#e20-storage-is-an-adapter-concern
- ../design/current-design.md#session-state-architecture-follows-the-rust-playbook
---

# ADR-0004: Keep session storage behind a core-owned port

## Decision

Session/workbench state storage is an outbound adapter concern.

The application/core capability owns the semantic storage port it consumes. Concrete adapters implement that port.

V0 uses SQLite as the runtime storage adapter because separate CLI invocations need to share workbench state. A memory implementation may still be useful as a test/fake adapter. The executable entry point/composition root selects the concrete implementation and injects it into the application service.

~~~text
application/core
  transition logic
  context/checkpoint semantics
  storage port
        ^
        |
adapters
  sqlite (v0 runtime)
  memory (optional test/fake)
        ^
        |
entry point
  constructs and injects
~~~

## Why the abstraction is justified now

Normally the project should avoid speculative abstractions.

This port is not speculative.

SQLite is required by the v0 CLI interaction because one-shot commands must share workbench state across process invocations. A memory implementation remains useful for tests and focused application-level exercises.

The port keeps SQLite mechanics out of transition logic and lets tests inject a simpler adapter without changing capability behavior.

## Port ownership

The port belongs beside the application capability that needs it, not in infrastructure.

The interface should describe Control Tower's semantic state needs, not generic persistence mechanics.

The original Store name was provisional. The pinned playbook assessment selects `WorkbenchQueries::read_checkpoint` and `WorkbenchWrites::record_checkpoint`, without a Store/UoW, because the existing callers independently read and record orchestration checkpoints. Do not introduce a generic Repository<T>, Store<T>, or application-wide persistence abstraction.

Exact methods should be derived from the application use cases during implementation rather than designed as a generic CRUD surface now.

## Adapter responsibilities

A storage adapter may:

- serialize/map state to SQLite,
- hold state in memory for tests/experiments,
- implement adapter-specific IO/error translation,
- perform adapter-local atomic writes/transactions when needed.

A storage adapter must not decide:

- which step is complete,
- how up/down changes checkpoints,
- whether verify passed,
- how source/candidate context is computed,
- what a failed transition means,
- which action should execute next.

Those are application/core rules.

## Composition root

The executable entry point is the composition root.

For the v0 product runtime it constructs the SQLite adapter and injects it into the application service.

Tests or focused experiments may construct a memory adapter against the same port.

Do not introduce a DI framework, service locator, container crate, or infrastructure-owned composition layer.

Explicit wiring is preferred.

## Ownership mechanics

Do not choose Arc merely because dependency injection is present.

If one service exclusively owns the port implementation, exclusive ownership such as Box<dyn Port> is sufficient. Use Arc only when the actual runtime has independent owners that require shared ownership.

The exact Rust type should follow the real implementation topology.

## V0 SQLite scope

SQLite is in v0 only to preserve Control Tower workbench state across ordinary short-lived CLI invocations.

This does not make v0 a crash-recovery system.

SQLite-specific schema, query details, adapter-local transactions, and mapping are implementation concerns for the SQLite adapter. They should stay as small as the actual application port requires.

If the Rust process crashes during an external mutation, the next invocation may only know the last Control Tower state successfully stored. Reconciliation of external side effects remains out of scope.

The application/core must not gain SQLite concepts merely because SQLite is the runtime adapter.

## Consequences

The initial implementation has one intentional seam despite being small.

That seam earns its cost because it protects the most likely near-term change while preserving the project's playbook:

- business/state-transition logic inward,
- IO/storage outward,
- explicit dependency injection at the edge,
- no framework machinery.

## Implementation refinement from the compliance correction

The concrete storage implementation lives in `control-tower-database`; non-database adapters live in `control-tower-infrastructure`. CLI Entry explicitly composes both with Application. The primary `control-tower` CLI exposes explicit `db bootstrap-local`, `db migrate-local`, and `db verify-local` operations that call Database-owned code; ordinary workbench commands only open previously prepared storage. This keeps the explicit lifecycle while avoiding a separate database executable. The current design documents the rusqlite default deviation, typed source-preserving error contracts, and intentional absence of a separate Domain concern. See [implemented architecture](../design/current-design.md#implemented-package-boundaries-and-persistence) and the [rule-level ledger](../research/playbook-compliance.md). Historical discovery statements remain historical.
