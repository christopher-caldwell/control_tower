---
id: CT-DESIGN
title: Current design and decision audit
type: design
status: maintained
created: '2026-09-30'
updated: '2026-10-01'
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
Application
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

Control Tower's Application layer owns a semantic outbound port for the session/workbench state it needs. A concrete storage adapter implements that port.

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

The Application must remain storage-independent. SQLite-specific schema, queries, transactions, and mapping stay inside the adapter.

A memory adapter may still be useful for tests, but it should not define v0 runtime behavior.

### Crash behavior remains deliberately weak

SQLite persistence does not turn crash recovery into a v0 goal.

If the Rust process crashes during an operation, Control Tower makes no promise to reconcile external side effects or reconstruct an interrupted transition. A later invocation uses whatever Control Tower state was last successfully stored. The developer can clean up/start over as needed.

Do not add crash journals, recovery protocols, external-state reconciliation, or structural-drift machinery merely because SQLite exists.

## Implemented package boundaries and persistence

The correction pass uses Rust Simpler Playbook revision `8d2ff6d906676020d671ba3b0674c0f8b58d9414`. The [rule-level evidence ledger](../research/playbook-compliance.md) records the assessment against implementation base `f967bda7` and the correction commit containing that ledger.

```text
control-tower-cli (binary-only Entry)
  -> control-tower-application
  -> control-tower-database -> control-tower-application
  -> control-tower-infrastructure -> control-tower-application
```

Database owns SQLite adapters, SQL/mapping, bootstrap, migrations and its separate operational executable. Infrastructure owns the `stage_discovery` and `executable_runner` capabilities. Neither constructs Application services or depends on its peer. `crates/cli/src/deps.rs` explicitly constructs four exclusively owned boxed ports and `Workbench`. No DI framework, service locator, generic dependency bag or shared composition package is present.

Domain is currently absent deliberately. `Direction` and `ExecutableRole` describe the invocation protocol; `Stage` describes discovered executable paths; `WorkbenchState` and `PendingTransition` describe the orchestrator's checkpoint and outstanding verification. Their validation checks whether that checkpoint can drive the discovered stage sequence. All current behavior sequences external capabilities or validates their orchestration inputs/results; no separate entity lifecycle, business eligibility, or Domain rule is implemented. Naming these records does not by itself earn a Domain package. If later features introduce genuine Domain meaning, the playbook requires its own package and foundational dependency assessment.

Application owns `WorkbenchQueries::read_checkpoint() -> Result<Option<WorkbenchState>, PersistenceError>` and `WorkbenchWrites::record_checkpoint(&WorkbenchState) -> Result<(), PersistenceError>`. An absent checkpoint is normal absence. Recording a checkpoint is one independently atomic singleton upsert, with no Application work between persistence statements. External scripts and multiple checkpoints are intentionally not one atomic transaction; no Store/UoW is earned. A stopped outcome retains the existing attempted in-memory state reporting behavior and does not establish that a failed checkpoint write persisted.

The Query adapter opens a read-only handle; the Write adapter opens an existing read-write handle to the same local primary file. No replica or cross-row/multi-instance guarantee is claimed.

### SQLite library and SQL default departure

`rusqlite` is retained as a **DEFAULT DEVIATION WITH JUSTIFICATION** from SQLx implementation/checking/tooling defaults. The actual program is synchronous: short-lived CLI processes discover files, run one child process at a time, and independently read/upsert one local checkpoint row. It has no async executor, concurrent request workload, server database, pool, or Application transaction. SQLx would require an executor and async adapter/port plumbing (or synchronous wrappers around that executor), plus a checked-schema/offline preparation workflow for a database that is created separately in each user workspace. That adds runtime/build tooling with no needed pooling or async-I/O benefit here. The cost of retaining rusqlite is losing compile-time query/schema checking; real SQLite mapping, absence, source-error and legacy migration tests exercise that boundary. This choice must be reconsidered if the actual workload changes.

Production Query/Write SQL remains external and feature-local under `crates/database/src/workbench/sql/`. Mapping reconciles SQLite integers/text with Application checkpoint values. Operational history/version inspection SQL is confined to Database operations; no SQL or driver type enters Application.

### Explicit local setup and migrations

`just db-bootstrap-local <workspace>` provisions the directory/file only. `just db-migrate-local <workspace>` requires that provisioned file, applies `migrations/0001-workbench-state.sql` in a SQLite transaction, and records version/history. Version 1 also adopts the original unversioned table without altering its shape or saved rows. Rerunning migration at version 1 is a no-op plus history verification. `just db-verify-local <workspace>` opens read-only and inspects supported history/version. All three invoke the Database-owned `control-tower-db` operational binary; ordinary `up`, `down`, and `status` never call them.

These commands accept an explicit local workspace, not an ambient database URL. There is no hosted/live database workflow. PostgreSQL logins, NOLOGIN object owners, role grants/default privileges and role assumption do not apply to SQLite. The read-write SQLite connection has file access, not a server role that denies DDL; separation of schema operations is architectural, and no server privilege isolation is claimed. Future hosted database operations would need separate intentional live inputs and their applicable ownership/privilege evidence.

### Error boundaries and inputs

Application owns opaque `PersistenceError`, `StageDiscoveryError` and `ExecutableRunError`, each retaining an underlying `Error` source. Real rusqlite and I/O errors are wrapped intact by their adapters; diagnostic context may add a source-preserving outer wrapper. Synthetic validation failures have typed message sources because no driver error exists to retain. Application never downcasts sources for behavior.

`status` exposes `StatusError`; `move_to` exposes `MoveToError`, with its additional invalid-target meaning. Process-start and checkpoint-write errors inside a stopped move also retain sources. One execution failure is referenced by both the execution event and stopped outcome using `Rc`, matching that actual single-threaded diagnostic sharing; injected dependencies remain `Box`, with no `Arc` introduced. Errors are not required to fabricate equality or clone driver sources.

`move_to` takes `MoveToInput` with public workspace, direction and target fields. Status has one direct workspace argument. Named result types and the existing UUID generation/handoff remain intact. The later [run-semantics validation](../research/run-semantics-validation.md) corrects active-transition reversal without changing these architecture boundaries or the SQLite schema.

## Completed position and active transition

Application records the last accepted stage count, one opaque UUID, and at most one pending stage/direction. The direction identifies the most recently successful mutation whose verifier is outstanding. The completed position may be on either adjacent side of that stage, including after reversal; it changes only after directional verification succeeds.

A request continuing the pending direction retries its verifier without replaying the mutation. A request in the opposite direction invokes the same stage's opposite mutation and optional verifier. Successful resolution clears pending state, then the sequential walk continues toward the target.

For example, completed 02 plus pending 03/up can resolve through verify-up to completed 03, or through 03/down and optional verify-down back to completed 02. If that verify-down fails, completed remains 02 and pending becomes 03/down; a later downward request retries only verify-down. Completed 03 plus pending 03/down supports the symmetric 03/up reversal.

SQLite stores this Application checkpoint and does not choose transitions. Earlier context-stack and patch proposals remain hypotheses, not implemented v0 requirements.

## V0 state handoff is intentionally minimal

Earlier exploration considered a generalized source/candidate context model.

That is **not** a v0 requirement.

The first formal discovery fixture only requires Control Tower to carry one opaque UUID from stage 1 into later stages. Discovery should use the smallest mechanism that makes that work and generalize only when a real use case demands it.

Source/candidate views may still turn out to be useful implementation concepts for directional verification, but they should earn their way through the concrete fixture rather than be treated as product requirements.

## Failure policy for v0

### Mutation exits nonzero

Do not change the completed step and do not automatically verify. Show stdout/stderr/exit status and stop.

### Mutation succeeds; verifier fails

Persist the active transition and UUID across normal CLI invocations. Allow matching verifier retry without rerunning the successful mutation, or explicit opposite-direction movement through the same stage's mutation and optional verifier. A failed reverse verifier leaves the original completed position intact and the reverse direction pending. Resolve that stage before walking farther toward the requested target.

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

No scheduler, authentication, hosting, DAG, built-in drivers, automatic mutation retries, crash recovery, concurrency control, structural-drift protection, transaction emulation, or expression language is required.

The high-level pre-discovery work is now sufficiently complete.

Use [First formal discovery brief](discovery-brief.md) for the next round.

The CLI/process-lifetime tension is resolved at the product level: SQLite is part of v0 so ordinary one-shot CLI commands can share workbench state. Discovery should not redesign this boundary; it should verify that the storage port keeps SQLite details outside application behavior.

The four-role step remains the leading mechanism subject to discovery. The initial proof requires only one UUID handoff, filesystem-only authoring, and no auxiliary-action abstraction.
