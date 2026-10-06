---
id: CT-DESIGN
title: Current design and decision audit
type: design
status: maintained
created: '2026-09-30'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- ../decisions/0006-loopback-web-ui.md
- ../decisions/0007-desktop-ui-shell.md
- ../research/2026-10-02-ui-reference-review.md
- ../research/playbook-compliance.md
- ../research/run-semantics-validation.md
- ../research/2026-10-01-guided-usage-findings.md
- ../research/2026-10-01-core-v0-completion.md
- ../reference/cli.md
- ../reference/stage-executables.md
---

# Current design and decision audit

This is the maintained implementation/intent summary. For commands, start with [getting started](../guides/getting-started.md), not the historical discovery inputs. Accepted ADRs remain the source of product decisions; this page does not introduce new approval.

## Product identity

Control Tower is a personal, local, migration-style workbench for user-owned executable actions. The author owns what operations mean. Control Tower owns ordered navigation, process execution, visible results, and the small amount of state needed between CLI invocations.

It is not an orchestration platform: no hosted control plane, login, scheduler, workers, or built-in HTTP/database action language.

## Core-v0 standing

The core is complete as a candidate for repeated local owner use under the bounded [completion standard and evidence](../research/2026-10-01-core-v0-completion.md). Real-process layered navigation, farther continuation after pending verification, both runnable examples and normal development gates support the current contract. This milestone adds no product semantics, accepted decision, managed context or recovery guarantee. Actual owner tickets and recurring ergonomics should drive the next iteration; retained findings and untried research scenarios are not a new completion backlog.

## Core model

Control Tower's vocabulary is **Workspace → Workflow → Stage**. A Workspace is a directory holding Workflows under `workflows/`; the browser UI treats its launch directory as the Workspace. A Workflow is the runnable unit: it owns its `stages/` and its own `.control_tower/state.sqlite3`. The CLI addresses one Workflow directly with `--workflow`. Stage executables receive that directory as `CONTROL_TOWER_WORKFLOW`.

```text
workspace/
  workflows/
    workflow-a/
      stages/
      .control_tower/state.sqlite3
    workflow-b/
      stages/
```

The shipped filesystem convention inside a Workflow is `stages/` with numbered directories and up to four role files:

```text
workflow/
  stages/
    001-create-fixture/
      up
      down
      verify-up
      verify-down
```

Both verifiers are optional. A stage needs at least one mutation; a verifier needs its matching mutation. Required missing mutations make traversal unavailable. Stage numbers are unique positive numeric prefixes, sorted numerically; gaps are allowed. No workflow YAML/TOML is used.

```text
up   -> optional verify-up   -> accept higher position
down -> optional verify-down -> accept lower position
```

The author is responsible for correctness and reversibility. A role name is not a sandbox or proof of side-effect freedom. Exact execution mechanics are in the [executable reference](../reference/stage-executables.md).

## Entry architecture: CLI and loopback browser UI

CLI is the implemented Application-driving adapter. It parses intent, calls Application use cases and renders results. Transition rules do not live in CLI. The executable entry point constructs concrete dependencies explicitly. See [ADR-0005](../decisions/0005-cli-first-driving-adapter.md).

[ADR-0006](../decisions/0006-loopback-web-ui.md) selects the graphical Entry: a built React frontend served by the Rust process over loopback HTTP. The platform-neutral `control-tower ui` command prints a plain local URL for manual browser opening, serves workflow inventory and selected-workflow movement/definition APIs, and sends SSE role observations. Production does not require Vite or Node.

The UI Entry composes Workbench inside the CLI package rather than shelling out to the CLI or changing Application ownership. Workspace scope comes from the launch directory, which must have a valid `control-tower.toml` with a nonempty `[workspace].label` and a `workflows/` directory. Startup inventory reads directory names under `workflows/` and uses the configured label. It does not open sibling databases or validate stages. Each selected workflow is composed and read inside a blocking Entry operation; an unprepared or malformed sibling does not prevent use of a healthy workflow. The UI does not bootstrap, migrate or repair storage. The normal executable embeds prebuilt assets. The UI is desktop-only; exact component styling remains an implementation detail.

Application exposes pure immediate movement choices on `WorkbenchStatus` and
`MoveOutcome`, using the same pending-transition derivation as verifier-failure
guidance. These choices identify mechanically available directions and actual
stage-number targets; they do not establish safe recovery after ambiguous
authored effects or failed persistence. HTTP maps the choices and current
checkpoint into its browser DTOs. React consumes the supplied immediate choices and owns
wording and emphasis. For Run to X and Run all it reuses the immediate upward choice
with a farther `target_stage`; Application's existing movement engine remains
responsible for validating that target, walking intermediate stages, verification,
checkpoint updates and stopping on failure. React does not add a separate attempted/confirmed checkpoint
presentation model.

The web implementation remains inside CLI/Entry, with internal modules for DTOs,
HTTP/static delivery, workflow inventory and selected-status mapping, in-process
snapshots, and the blocking movement/inspection bridge. Entry composition stays
explicit in `deps.rs`; the test harness is separate. This split adds no Application
services, shared Workbench requirements, or persistence model.

Application offers synchronous observations before and after each actual role attempt. CLI renders captured output/results before the next role is attempted. The UI maps those observations to SSE and keeps the latest in-process attempt for the selected workflow. It converts each completed role's captured stdout/stderr to separate displayable text in the snapshot, so completed mutation output remains available while a later verifier runs. The Application outcome and Infrastructure runner retain their existing raw bytes. True byte-by-byte process-output streaming is deferred until real use demonstrates that role-level running/completed state plus finished output is insufficient.

The `control-tower db` subcommands perform explicit operational database setup through the primary CLI. They call Database-owned operations before workbench composition; ordinary workbench commands do not bootstrap or migrate storage.

## Selected UI shell: desktop workspace -> workflow -> stage workbench

[ADR-0007](../decisions/0007-desktop-ui-shell.md) fixes the first UI's information architecture before implementation. The launch directory is the v0 **Workspace** context; the UI discovers and switches among workspace-local **Workflows**. Switching to unrelated Workspaces from inside the running UI is deferred.

The primary desktop shell keeps three contexts visible together:

```text
left: workspace-local workflows
center: ordered vertical stages
right: selected-stage inspector
bottom: declarative progression/recovery actions
```

The left and right rails are visible by default and may be manually collapsed. Their widths are fixed in CSS, and collapse state lasts only while the page is open. Stage selection is primarily inspection: it does not execute anything or mutate the checkpoint, and it does not alter the immediate next transition. A selected future stage may also supply the target for an explicit Run to Stage X action.

Primary actions describe intent such as **Run next**, **Run to Stage 3 · Add suffix**, **Run all**, **Retry verification**, or **Back out 003 -> 002**. Mechanical role detail such as `up -> verify-up` can appear as explanatory subtext. Application remains authoritative for the available actions.

The interface is explicitly desktop-only. Responsive breakpoints must not turn the workbench into stacked cards, hamburger navigation, temporary mobile drawers, bottom sheets or a single-pane drill-down flow. Manual rail collapse is a user choice, not responsive behavior.

The [UI reference review](../research/2026-10-02-ui-reference-review.md) preserves the Dagu, Inngest, Decagon and Playwright source screenshots that informed density, simultaneous context and inspector behavior.

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

Checkpoint proposals are published to confirmed Application state only after a successful write. A save failure returns the latest confirmed state, attempted update, original error and all collected role results; further roles/writes stop. An absent initial row means default baseline, not a fabricated saved row. Last-confirmed bookkeeping does not prove current database contents after an ambiguous storage error.

### SQLite library and SQL default departure

The compliance pass retained `rusqlite` as a documented DEFAULT deviation: this is a synchronous, short-lived CLI with blocking script execution and one local checkpoint read/upsert at a time. The tradeoff is losing SQLx compile-time query checking while avoiding an otherwise unused async execution/tooling path. Feature-local external SQL and real SQLite mapping/source tests provide runtime evidence, not compile-time checking. The full justification and its limits remain in the [ledger](../research/playbook-compliance.md#default-deviations-and-sql-inventory).

### Explicit local setup and migrations

`bootstrap-local` provisions the file; `migrate-local` applies the versioned schema; `verify-local` reads the supported version/history. Ordinary `up`, `down`, and `status` do not call bootstrap/migrate. The current migration can adopt the earlier v0 table and preserve its saved rows. Operational schema history is distinct from a history of user-authored stage executions.

PostgreSQL server roles and privileges do not apply to this embedded file. No server-style DDL privilege isolation is claimed. See [database commands](../reference/cli.md#database-operations), [setup](../guides/getting-started.md#3-prepare-and-validate-the-workflow), and the Database-owned operations implementation.

### Error boundaries and inputs

Application owns opaque `PersistenceError`, `StageDiscoveryError`, and `ExecutableRunError`, preserving implementation sources. `status` exposes `StatusError`; `move_to` exposes `MoveToError` and takes a named `MoveToInput`. CLI renders Application meaning without inspecting database codes. Diagnostic sharing uses `Rc`; service dependencies remain exclusively owned `Box` values. Source-retention tests and the exact scoped assessment remain in the compliance record.

## Completed position and active transition

Application stores the last accepted stage count, one UUID, and at most one pending stage/direction. Pending direction identifies the most recently successful mutation awaiting verification. The completed position may be on either adjacent side of that stage after a reversal.

Continuing the pending direction retries only its verifier. Opposite-direction movement runs the same stage's opposite mutation and optional verifier. Successful resolution clears pending state, then the sequential walk continues toward the target.

For example, completed 2 plus pending 3/up can become completed 3 through verify-up, or return to completed 2 through 3/down and optional verify-down. If that reverse check fails, completed remains 2 with pending 3/down; a later downward command retries only that check. Completed 3 plus pending 3/down supports the symmetric upward reversal.

The [run-semantics validation](../research/run-semantics-validation.md) records the implementation correction and tests. The [user guide](../guides/verification-and-navigation.md) demonstrates the loop. Earlier checkpoint-stack/source/candidate/patch designs are not the shipped v0 state model.

## V0 state handoff is intentionally minimal

The shipped runner generates one UUID before a run's first mutation and supplies it to every role. The original UUID-file sample uses it to name a file; it does not generate and publish an ID back to the runner. Stdout is output for the user, not a parsed state-update channel.

This distinction is recorded in [ADR-0003's implementation observation](../decisions/0003-session-state-and-process-io.md#current-implementation-observation). The UUID-file sample proves a shared runner token and navigation. The optional [generated-ID sample](../../examples/simple/workflows/generated-id/README.md) demonstrates an author-owned JSON handoff and separate application SQLite database through the existing executable contract; it adds Python only as an example prerequisite. Managed script-produced context remains unimplemented; the example does not accept ADR-0003's broader proposals.

The UUID persists while verification is pending and is cleared on successful settlement at baseline 0. SQLite remains prepared for the next run.

A failed first mutation retains its preallocated UUID at baseline without pending verification. A down-0 no-op does not reset that UUID; a retry reuses it. Optional checks are rediscovered each invocation, so removing a pending verifier can accept without rechecking or mutation replay. Settled-target movement and status remain metadata-only with respect to external fixture correctness.

## Failure policy for v0

A nonzero mutation stops the walk before its verifier or later stages. A successful mutation with failed verification retains the pending state for explicit retry or reversal across normal CLI exits. A failed reverse verifier retains the original completed position and the new pending direction.

A Rust crash or failed checkpoint write carries no external-state reconciliation guarantee. Scripts may have changed external systems even if local bookkeeping was not updated. SQLite persistence is not a transaction around arbitrary executable side effects.

Movement and status report actual numeric stage identifiers/labels for completed and pending positions, keeping persistence's internal count/index representation out of user-facing identity. Verifier-specific next steps depend on this invocation's failure, not just an old pending checkpoint after a failed reverse mutation or save. There is no automatic write retry, mutation retry or recovery flow.

## No concurrency or structural-drift machinery

This is one user's one-instance workbench. No multi-instance locking, drift detection, or reconciliation is implemented. Stage-directory changes during a stored run are the author's responsibility. **Restarting does not clear SQLite state**; finish the run before structural edits or start a fresh workflow after handling external effects yourself.

## Deferred reset escape hatch

A workflow-wide reset executable remains a future idea, not a current command. Deleting bookkeeping is not equivalent to running author-owned reversal scripts.

## What still has not earned scope

The graphical UI delivery architecture and desktop shell in ADR-0006/ADR-0007 are implemented as a platform-neutral loopback browser workbench. The frontend uses three simultaneously visible regions with deliberate rail collapse; it preserves selection/checkpoint distinction and escaped role-definition text. Declarative movement, SSE role observations, completed text output and outcome-specific recovery are implemented in the UI Entry. Native app packaging, workspace switching, persistent execution history, mobile/tablet responsive behavior, WebSockets and live byte-by-byte stdout/stderr streaming remain outside the selected scope. Helper ecosystems, generalized context protocol, automatic mutation retry, crash recovery, concurrent-instance coordination, structural-drift protection and an external transaction system also remain outside the implemented scope.

The first discovery and correction rounds have implementation evidence. The next useful input is actual use, not replaying the historical discovery queue as setup work. [Open questions](open-questions.md) keeps that future work separate; the [discovery brief](discovery-brief.md) and [earlier probe](three-step-workspace.md) remain historical inputs.
