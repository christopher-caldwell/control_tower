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
          +--> SQLite adapter (v0)
          +--> memory adapter (optional test/fake)
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
SQLite v0       memory test/fake
~~~

The v0 composition root constructs the SQLite adapter and injects it into the application service. A memory implementation may still be useful for tests or focused experiments, but it is no longer the runtime product adapter.

No transition logic, context-checkpoint math, verification rules, or patch semantics belong in the adapter. The adapter stores/retrieves the state requested by the application port.

This is an accepted architecture decision; see [ADR-0004](../decisions/0004-session-storage-port-and-adapters.md).

## V0 behavior: SQLite adapter

SQLite is now part of v0 because the CLI is expected to support normal short-lived invocations while preserving workbench state between commands.

For example:

~~~text
control_tower up ...
# process exits normally

control_tower down ...
# new process reads the same workspace state
~~~

This is a product-ergonomics requirement, not a durability initiative.

The application/core must remain storage-independent. SQLite-specific schema, queries, transactions, and mapping stay inside the adapter.

A memory adapter may still be useful for tests, but it should not define v0 runtime behavior.

### Crash behavior remains deliberately weak

SQLite persistence does not turn crash recovery into a v0 goal.

If the Rust process crashes during an operation, Control Tower makes no promise to reconcile external side effects or reconstruct an interrupted transition. A later invocation uses whatever Control Tower state was last successfully stored. The developer can clean up/start over as needed.

Do not add crash journals, recovery protocols, external-state reconciliation, or structural-drift machinery merely because SQLite exists.

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

### Rust process exits/crashes

Normal CLI process exit preserves state through SQLite.

A Rust crash has no recovery guarantee. The next invocation sees whatever Control Tower state was last successfully stored; external side effects may differ and remain the author's responsibility.

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

Use [First formal discovery brief](discovery-brief.md) for the next round.

The CLI/process-lifetime tension is resolved at the product level: SQLite is part of v0 so ordinary one-shot CLI commands can share workbench state. Discovery should not redesign this boundary; it should verify that the storage port keeps SQLite details outside application behavior.

The four-role step remains the leading mechanism subject to discovery. The initial proof requires only one UUID handoff, filesystem-only authoring, and no auxiliary-action abstraction.
