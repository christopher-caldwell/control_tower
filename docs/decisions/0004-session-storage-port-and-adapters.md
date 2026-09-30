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

V0 uses an in-memory adapter for simplicity. SQLite is the planned fast-follow adapter. The executable entry point/composition root selects the concrete implementation and injects it into the application service.

~~~text
application/core
  transition logic
  context/checkpoint semantics
  storage port
        ^
        |
adapters
  memory
  sqlite (later)
        ^
        |
entry point
  constructs and injects
~~~

## Why the abstraction is justified now

Normally the project should avoid speculative abstractions.

This port is not speculative: two concrete storage implementations are already known.

- memory is the deliberate v0 choice,
- SQLite is an expected near-term replacement/alternative.

Without the port, a later SQLite move would risk coupling transition logic to storage mechanics. With the port, the composition root can swap the adapter while the capability logic remains unchanged.

## Port ownership

The port belongs beside the application capability that needs it, not in infrastructure.

The interface should describe Control Tower's semantic state needs, not generic persistence mechanics.

Prefer a name such as SessionStateStore / WorkbenchStateStore only if it matches the final capability language. Do not introduce a generic Repository<T>, Store<T>, or application-wide persistence abstraction.

Exact methods should be derived from the application use cases during implementation rather than designed as a generic CRUD surface now.

## Adapter responsibilities

A storage adapter may:

- hold state in memory,
- serialize/map state to SQLite later,
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

For v0 it constructs the memory adapter and injects it into the application service.

Later it can construct a SQLite adapter instead and inject the same port.

Do not introduce a DI framework, service locator, container crate, or infrastructure-owned composition layer.

Explicit wiring is preferred.

## Ownership mechanics

Do not choose Arc merely because dependency injection is present.

If one service exclusively owns the port implementation, exclusive ownership such as Box<dyn Port> is sufficient. Use Arc only when the actual runtime has independent owners that require shared ownership.

The exact Rust type should follow the real implementation topology.

## V0 behavior versus SQLite fast follow

The memory adapter means a process restart starts with empty Control Tower state.

The later SQLite adapter can make the same logical state durable.

SQLite-specific schema, migrations, resume behavior, and transaction details are explicitly not part of the first pass.

When SQLite work starts, its adapter should be added without moving SQLite concepts into application/core.

## Consequences

The initial implementation has one intentional seam despite being small.

That seam earns its cost because it protects the most likely near-term change while preserving the project's playbook:

- business/state-transition logic inward,
- IO/storage outward,
- explicit dependency injection at the edge,
- no framework machinery.
