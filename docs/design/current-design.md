---
id: CT-DESIGN
title: Current design and decision audit
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../history/2026-09-30-initial-design.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- discovery-brief.md
- three-step-workspace.md
---

# Current design and decision audit

## Product identity

Control Tower remains a personal, local, migration-style workbench for arbitrary user-owned executable actions. The developer owns the work. Control Tower owns ordered navigation, process execution, visible results, and a small session-state model.

It is intentionally not an orchestration product.

## Core model

The strongest current step shape remains:

~~~text
step/
  up
  down
  verify-up     # optional
  verify-down   # optional
~~~

A directional mutation changes the completed step only after its optional directional verifier succeeds.

~~~text
forward:  up   -> verify-up   -> complete higher step
backward: down -> verify-down -> complete lower step
~~~

The author remains responsible for the semantics of every executable.

## V0 entry architecture: CLI only

The CLI is the first and only driving adapter for v0.

Conceptually:

~~~text
user
  |
  v
CLI adapter
  |
  | invokes application use cases
  v
application/core
  |
  +--> process execution port/adapter
  |
  +--> session-state port
          |
          +--> memory adapter (v0)
          +--> SQLite adapter (later)
~~~

The CLI owns command-line concerns: parsing arguments, selecting an application use case, and rendering results/errors for a terminal.

The CLI does **not** own transition rules, checkpoint/context behavior, verification semantics, or storage mechanics.

The executable entry point can also act as the composition root: construct the concrete adapters, construct the application capability/service, and pass that service to the CLI-facing layer.

No abstraction for “all possible frontends” is needed now. When a Tauri or HTTP entry layer is added later, it should become another driving adapter calling the same application use cases.

See [ADR-0005](../decisions/0005-cli-first-driving-adapter.md).

## Session-state architecture follows the Rust playbook

The **behavioral logic must not depend on in-memory storage**.

Control Tower's application/core layer owns a semantic outbound port for the session/workbench state it needs. A concrete storage adapter implements that port.

Conceptually:

~~~text
entry point / composition root
          |
          | chooses concrete adapter
          v
application service
          |
          | application-owned session-state port
          v
storage adapter
    |               |
memory v0       SQLite fast follow
~~~

The v0 composition root constructs the in-memory adapter and injects it into the application service. A later SQLite implementation should satisfy the same core-owned port and be selected at the entry point instead.

No transition logic, context-checkpoint math, verification rules, or patch semantics belong in the adapter. The adapter stores/retrieves the state requested by the application port.

This is an accepted architecture decision; see [ADR-0004](../decisions/0004-session-storage-port-and-adapters.md).

## V0 behavior: memory adapter

The initial adapter is intentionally in-memory.

With that adapter, Rust-process lifetime is effectively the session lifetime:

~~~text
process exits/crashes
  -> memory adapter disappears
  -> next launch begins with empty state
~~~

That is acceptable v0 behavior.

The application/core should not be written *as if* memory is intrinsic. The same transition service should operate against a later SQLite adapter without knowing which adapter was injected.

## SQLite is a fast follow, not first-pass scope

SQLite is expected soon after the memory implementation, which is why the storage port has earned its place now.

That does **not** justify designing a SQLite schema, migration system, transaction layer, durability policy, or resume UX in the first pass.

When SQLite work begins, the adapter will own SQLite-specific mechanics. The application contract should change only if the actual capability needs change.

One product question will become relevant then: whether startup resumes the last stored workbench session or intentionally starts fresh. That question is deferred until the SQLite adapter exists; it does not belong in the memory implementation.

## Composition and dependency direction

The intended dependency direction is inward:

~~~text
application/core
    owns state semantics and outbound storage port
          ^
          |
adapters
    implement the port
          ^
          |
entry point
    constructs concrete adapter and injects it
~~~

The entry point is the composition root.

Do not introduce:

- a DI framework,
- service locator,
- generic Repository<T>/Store<T>,
- adapter selection inside application logic,
- infrastructure types crossing into the core,
- a bootstrap/container abstraction merely to avoid explicit construction.

Use explicit constructor injection. If a dependency has one owner, exclusive ownership is preferable; shared ownership such as Arc should only appear when independent owners actually require it.

Exact crate/module layout and pointer types can follow implementation pressure. The dependency rule is the important part.

## Context checkpoints remain a domain/application concern

During a running session, completed-step checkpoints and one active directional transition remain the strongest state model.

Example:

~~~text
completed 01:
  user_id: 123

completed 02:
  user_id: 123
  record_id: 456
~~~

The application logic decides how a forward patch builds a candidate context and how a verified down returns to a lower checkpoint.

The storage adapter does **not** calculate those transitions. It stores the application state through the port.

## V0 state handoff is intentionally minimal

Earlier exploration considered a generalized source/candidate context model.

That is **not** a v0 requirement.

The first formal discovery fixture only requires Control Tower to carry one opaque UUID from stage 1 into later stages. Discovery should use the smallest mechanism that makes that work and generalize only when a real use case demands it.

Source/candidate views may still turn out to be useful implementation concepts for directional verification, but they should earn their way through the concrete fixture rather than be treated as product requirements.

## Failure policy for v0

### Mutation exits nonzero

Do not change the completed step and do not automatically verify. Show stdout/stderr/exit status and stop.

### Mutation succeeds; verifier fails

Keep the active transition in the injected state store for the life of the current session. Allow verifier retry and inspection without rerunning the mutation.

### Rust process exits/crashes with memory adapter

State is lost. Start over.

No crash-recovery feature is required for v0.

## No concurrency or structural-drift machinery

This remains a one-user, one-instance workbench.

No locks or race-prevention system is required.

Changing step directories while a session exists is the author's responsibility. Restart if a clean model is desired.

## Deferred reset escape hatch

A future workspace-level reset executable may provide an explicit author-owned “back everything out” escape hatch.

It remains deferred.

## What still has not earned scope

No scheduler, authentication, hosting, DAG, built-in drivers, retries, crash recovery, concurrency control, structural-drift protection, transaction emulation, or expression language is required.

The high-level pre-discovery work is now sufficiently complete.

Use [First formal discovery brief](discovery-brief.md) for the next round. The most important unresolved point is the interaction between an in-memory state adapter and CLI process lifetime: a one-shot CLI command cannot preserve memory state for a later command after the process exits. Formal discovery should test the smallest CLI interaction that preserves the intended workbench loop.

The four-role step remains the leading mechanism subject to discovery. The initial proof requires only one UUID handoff, filesystem-only authoring, and no auxiliary-action abstraction.
