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
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- ../research/playbook-compliance.md
- ../research/run-semantics-validation.md
- ../reference/cli.md
- ../reference/stage-executables.md
---

# Current design and decision audit

This is the maintained implementation/intent summary. For commands, start with [getting started](../guides/getting-started.md), not the historical discovery inputs. Accepted ADRs remain the source of product decisions; this page does not introduce new approval.

## Product identity

Control Tower is a personal, local, migration-style workbench for user-owned executable actions. The author owns what operations mean. Control Tower owns ordered navigation, process execution, visible results, and the small amount of state needed between CLI invocations.

It is not an orchestration platform: no hosted control plane, login, scheduler, workers, or built-in HTTP/database action language.

## Core model

The shipped filesystem convention is `stages/` with numbered directories and up to four role files:

```text
workspace/
  stages/
    001-create-fixture/
      up
      down
      verify-up
      verify-down
```

Both verifiers are optional. A stage needs at least one mutation; a verifier needs its matching mutation. Required missing mutations make traversal unavailable. Stage numbers are unique positive numeric prefixes, sorted numerically; gaps are allowed. No workspace YAML/TOML is used.

```text
up   -> optional verify-up   -> accept higher position
down -> optional verify-down -> accept lower position
```

The author is responsible for correctness and reversibility. A role name is not a sandbox or proof of side-effect freedom. Exact execution mechanics are in the [executable reference](../reference/stage-executables.md).

## V0 entry architecture: CLI only

CLI is the only v0 Application-driving adapter. It parses intent, calls Application use cases and renders results. Transition rules do not live in CLI. The executable entry point constructs concrete dependencies explicitly; future Tauri/HTTP delivery would call the same Application behavior.

The separate `control-tower-db` binary performs operational database setup. It is not a second workbench UI or an additional application transport. See [ADR-0005](../decisions/0005-cli-first-driving-adapter.md).

## Session-state architecture follows the Rust playbook

Application owns the storage capability contracts. SQLite is the v0 runtime adapter because one-shot CLI commands need state to survive process exit. A memory adapter can be used for testing; storage representation does not define transition semantics.

```text
CLI Entry -> Application use cases
              -> inward-owned Query/Write and process/discovery ports
                   <- Database / Infrastructure implementations
```

The composition root chooses implementations. SQLite stores/maps checkpoints; it does not decide whether verification passed or which stage should run. See [ADR-0004](../decisions/0004-session-storage-port-and-adapters.md).

## Implemented package boundaries and persistence

The architecture correction used Rust Simpler Playbook revision `8d2ff6d906676020d671ba3b0674c0f8b58d9414`. Its [compliance ledger](../research/playbook-compliance.md) preserves the scoped assessment and worker execution record.

```text
control-tower-cli (binary-only Application Entry)
  -> control-tower-application
  -> control-tower-database -> control-tower-application
  -> control-tower-infrastructure -> control-tower-application
```

Database owns SQLite, SQL/mapping and schema operations. Infrastructure owns stage discovery and executable invocation. Neither outer package assembles Application services or depends on its peer. `crates/cli/src/deps.rs` constructs four boxed capability implementations and the `Workbench` service. There is no DI framework, service locator or global dependency bag.

Domain is deliberately absent in the current implementation: the modeled types represent invocation choices, executable metadata and orchestration progress, rather than separate business entities or eligibility rules. That is the scoped assessment from the architecture pass, not a claim that developer tools can never have a Domain. New genuine Domain behavior would require reassessing the playbook boundary.

Application uses `WorkbenchQueries::read_checkpoint()` for an independent read and `WorkbenchWrites::record_checkpoint()` for one independent singleton upsert. Absence is `Option`; no Application-managed transaction is claimed across scripts or checkpoints, so no Store/UoW was added. The Query adapter uses a read-only connection and the Write adapter an existing read-write connection to the same local file.

### SQLite library and SQL default departure

The compliance pass retained `rusqlite` as a documented DEFAULT deviation: this is a synchronous, short-lived CLI with blocking script execution and one local checkpoint read/upsert at a time. The tradeoff is losing SQLx compile-time query checking while avoiding an otherwise unused async execution/tooling path. Feature-local external SQL and real SQLite mapping/source tests provide runtime evidence, not compile-time checking. The full justification and its limits remain in the [ledger](../research/playbook-compliance.md#default-deviations-and-sql-inventory).

### Explicit local setup and migrations

`bootstrap-local` provisions the file; `migrate-local` applies the versioned schema; `verify-local` reads the supported version/history. Ordinary `up`, `down`, and `status` do not call bootstrap/migrate. The current migration can adopt the earlier v0 table and preserve its saved rows. Operational schema history is distinct from a history of user-authored stage executions.

PostgreSQL server roles and privileges do not apply to this embedded file. No server-style DDL privilege isolation is claimed. See [database commands](../reference/cli.md#database-operations), [setup](../guides/getting-started.md#set-up-that-workspaces-database), and the Database-owned operations implementation.

### Error boundaries and inputs

Application owns opaque `PersistenceError`, `StageDiscoveryError`, and `ExecutableRunError`, preserving implementation sources. `status` exposes `StatusError`; `move_to` exposes `MoveToError` and takes a named `MoveToInput`. CLI renders Application meaning without inspecting database codes. Diagnostic sharing uses `Rc`; service dependencies remain exclusively owned `Box` values. Source-retention tests and the exact scoped assessment remain in the compliance record.

## Completed position and active transition

Application stores the last accepted stage count, one UUID, and at most one pending stage/direction. Pending direction identifies the most recently successful mutation awaiting verification. The completed position may be on either adjacent side of that stage after a reversal.

Continuing the pending direction retries only its verifier. Opposite-direction movement runs the same stage's opposite mutation and optional verifier. Successful resolution clears pending state, then the sequential walk continues toward the target.

For example, completed 2 plus pending 3/up can become completed 3 through verify-up, or return to completed 2 through 3/down and optional verify-down. If that reverse check fails, completed remains 2 with pending 3/down; a later downward command retries only that check. Completed 3 plus pending 3/down supports the symmetric upward reversal.

The [run-semantics validation](../research/run-semantics-validation.md) records the implementation correction and tests. The [user guide](../guides/verification-and-navigation.md) demonstrates the loop. Earlier checkpoint-stack/source/candidate/patch designs are not the shipped v0 state model.

## V0 state handoff is intentionally minimal

The shipped runner generates one UUID before a run's first mutation and supplies it to every role. The sample stage 1 uses it to name a file; stage 1 does not currently generate and publish an ID back to the runner. Stdout is output for the user, not a parsed state-update channel.

This distinction is recorded in [ADR-0003's implementation observation](../decisions/0003-session-state-and-process-io.md#current-implementation-observation). The sample proves a shared runner token and navigation, not arbitrary API-generated-ID capture. A richer script-produced handoff remains unimplemented and should not be invented during a documentation pass.

The UUID persists while verification is pending and is cleared on successful settlement at baseline 0. SQLite remains prepared for the next run.

## Failure policy for v0

A nonzero mutation stops the walk before its verifier or later stages. A successful mutation with failed verification retains the pending state for explicit retry or reversal across normal CLI exits. A failed reverse verifier retains the original completed position and the new pending direction.

A Rust crash or failed checkpoint write carries no external-state reconciliation guarantee. Scripts may have changed external systems even if local bookkeeping was not updated. SQLite persistence is not a transaction around arbitrary executable side effects.

## No concurrency or structural-drift machinery

This is one user's one-instance workbench. No project-specific locks, drift detection or reconciliation is implemented. Stage-directory changes during a stored run are the author's responsibility. **Restarting does not clear SQLite state**; finish the run before structural edits or start a fresh workspace after handling external effects yourself.

## Deferred reset escape hatch

A workspace-wide reset executable remains a future idea, not a current command. Deleting bookkeeping is not equivalent to running author-owned reversal scripts.

## What still has not earned scope

No UI/Tauri/HTTP, helper ecosystem, generalized context protocol, automatic mutation retry, crash recovery, concurrent-instance coordination, structural-drift protection or external transaction system is selected for this implementation.

The first discovery and correction rounds have implementation evidence. The next useful input is actual use, not replaying the historical discovery queue as setup work. [Open questions](open-questions.md) keeps that future work separate; the [discovery brief](discovery-brief.md) and [earlier probe](three-step-workspace.md) remain historical inputs.
