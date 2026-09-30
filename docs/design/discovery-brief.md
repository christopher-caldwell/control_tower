---
id: CT-DISCOVERY-001
title: First formal discovery brief
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
discovery_status: ready
sources:
- current-design.md
- three-step-workspace.md
- open-questions.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- ../history/2026-09-30-initial-design.md#e22-first-formal-discovery-seed
---

# First formal discovery brief

## Purpose

This is the seed for Control Tower's first formal discovery round.

Discovery should challenge the current direction, not merely translate it into code. The goal is to determine the smallest coherent v0 that proves the workbench interaction and respects the established Rust architecture boundaries.

Do not expand the product into an orchestration platform.

## Product intent

Control Tower is a personal, local workbench for walking arbitrary user-owned executable actions in migration-style order.

The motivating loop is:

~~~text
prepare test state
move forward
verify
inspect
change application code
move backward
try again
~~~

Control Tower owns navigation and execution bookkeeping. The author owns what each executable actually does.

## Settled constraints

These are not discovery questions unless direct evidence shows a contradiction.

### Product boundary

- local-only personal tool,
- Rust implementation,
- not hosted,
- no login/auth system,
- no scheduler,
- no worker fleet,
- no DAG/general workflow engine,
- no built-in HTTP/Postgres/Node execution model in core,
- user-owned executables selected by normal executable/shebang behavior.

### Architecture

Follow the owner's Rust hexagonal/ports-and-adapters playbook.

- CLI is the only v0 driving adapter.
- Application/core owns business/workbench behavior.
- Storage is an outbound adapter concern behind an application-owned semantic port.
- Memory is the v0 state adapter.
- SQLite is a fast-follow adapter, not first-pass implementation scope.
- The executable entry point is the composition root and wires concrete adapters explicitly.
- Do not introduce a DI framework, generic repository, service locator, or infrastructure-owned behavior.
- Future Tauri/HTTP entry layers should invoke the same application use cases.

### Scope discipline

- no concurrency/multi-instance protection,
- no crash recovery for v0,
- no structural-drift detection if step directories are changed mid-session,
- no migration checksums/history database,
- no attempt to repair external side effects after Control Tower itself crashes,
- optional workspace-wide reset is a future escape-hatch idea only.

## Leading step convention

The leading mechanism, explicitly subject to discovery and change, is up to four executable roles per numbered step:

~~~text
001-something/
  up
  down
  verify-up
  verify-down
~~~

Both verifiers are optional.

The leading directional behavior is:

~~~text
up -> optional verify-up -> complete higher step

down -> optional verify-down -> complete lower step
~~~

A verifier checks the direction-specific mutation. It should not perform the mutation itself.

If mutation succeeds but its verifier fails, retain enough session state to let the developer retry the verifier without automatically rerunning the mutation.

This convention survived the current conceptual gauntlet, but formal discovery should still test whether it remains the smallest useful mechanism.

## Filesystem-first authoring

Start with filesystem convention only.

Do not assume YAML/TOML/workspace config is required.

The discovery prototype should first see how far numbered step directories, fixed executable-role names, and shebangs can go. Introduce metadata/config only if a concrete problem cannot be handled cleanly by the filesystem convention.

## Concrete three-step discovery fixture

Use a temporary directory and one UUID-named file. This fixture intentionally tests state passing without requiring HTTP, a database, or a generalized context model.

### Stage 1 — create the fixture

**up**

- generate a UUID,
- create a file whose filename is that UUID,
- make the UUID available to later stages.

**verify-up**

- assert that the UUID-named file exists.

**down**

- delete the UUID-named file.

**verify-down**

- assert that the UUID-named file does not exist.

### Stage 2 — add first state

**up**

- use the same UUID to find the file,
- make its contents exactly or effectively `hello`.

**verify-up**

- assert that the UUID file contains the expected `hello` state.

**down**

- return the file to the stage-1 content state: empty.

**verify-down**

- assert that the UUID file is empty.

### Stage 3 — extend the state

**up**

- use the same UUID file,
- add ` to you` so the resulting content is `hello to you`.

**verify-up**

- assert that `hello to you` is present as expected.

**down**

- remove only the stage-3 contribution, ` to you`,
- leave the stage-2 `hello` state intact.

**verify-down**

- assert that the file is back to the stage-2 `hello` state.

## What this fixture is intended to prove

The first discovery fixture should demonstrate, or falsify, these ideas:

1. Numbered filesystem steps are enough to describe a useful workspace.
2. A step can be navigated up and down without Control Tower understanding its semantics.
3. Directional verification can gate completion without a built-in assertion language.
4. An up mutation can be retried at the verification layer without automatically rerunning the mutation.
5. Down can restore the immediately previous logical state.
6. One opaque value produced in stage 1 can be carried into later stages.
7. The application behavior can remain independent of the concrete state-storage adapter.
8. The CLI can expose the workbench lifecycle without putting application logic in the CLI adapter.

## Minimal v0 state-passing requirement

Do **not** begin discovery by designing a generalized context/key-value system.

The concrete fixture only requires Control Tower to carry one opaque identifier: the UUID created by stage 1.

Discovery may choose the smallest reasonable transport/protocol for that proof.

Potential mechanisms can be tried and compared, but none is prescribed here. The important requirement is that stage 2/3 and their verifiers can access the same UUID without rediscovering it manually.

If this experiment demonstrates a need for richer state, document the concrete need before generalizing.

## CLI requirement, not CLI syntax

The exact command surface is not frozen.

An illustrative shape is:

~~~text
control_tower up --stage 3 --workspace test_0123
~~~

The important capability is to choose a workspace and request movement to a target stage/direction.

Discovery may change names, flags, or interaction style freely.

### Important memory/CLI tension

The v0 storage adapter is in-memory.

A one-shot CLI process such as:

~~~text
control_tower up ...
# process exits

control_tower down ...
# new process
~~~

cannot retain workbench state between those invocations without durable storage.

Formal discovery must explicitly test this constraint.

Plausible options include:

- a long-lived interactive CLI session/REPL that keeps the memory adapter alive while the developer moves up/down,
- limiting one-shot commands to operations that can derive all necessary state within one process invocation,
- or deciding that one-shot stepwise CLI ergonomics justify pulling the SQLite adapter forward.

Do not pick an option merely to preserve an earlier illustrative command. Preserve the product goal and architecture boundaries instead.

## Auxiliary actions

Earlier discussion used “auxiliary action” to mean a non-migration executable such as an arbitrary Inspect button.

That abstraction has not earned v0 scope.

The first discovery fixture does not require an `actions/` directory. Inspection can be achieved through normal CLI/status output, verifier output, or direct filesystem inspection during the experiment.

If discovery finds a concrete need for user-authored non-stage actions, document it and then reintroduce the concept.

## Questions formal discovery should answer

### A. Does the four-role step remain the smallest useful authoring contract?

Test missing verifiers, failed verify-up, failed verify-down, and moving repeatedly 1 -> 2 -> 3 -> 2 -> 3.

### B. What is the smallest state handoff needed for the UUID fixture?

Do not solve future structured data problems prematurely.

### C. How should an in-memory CLI session actually feel?

This is the biggest unresolved interaction/architecture tension.

The result should make it easy to:

- open/select the workspace,
- move toward a target stage,
- see what ran and failed,
- retry directional verification without rerunning a successful mutation,
- move backward.

### D. Is filesystem convention enough?

Only add workspace metadata/config if the experiment produces a concrete reason.

### E. Does the port/adapter architecture stay clean under the real use cases?

The memory adapter should be swappable without changing transition behavior. The CLI should remain a driving adapter rather than becoming the application.

## What discovery should not decide yet

Unless the fixture forces the issue, do not spend discovery on:

- SQLite schema,
- Tauri,
- HTTP server,
- visual UI,
- helper ecosystem,
- persistent crash recovery,
- multi-instance behavior,
- scheduler/auth/hosting,
- public CLI compatibility,
- generic plugin architecture,
- generalized structured context types.

## Discovery completion signal

The first formal discovery is successful when it can explain a coherent minimal v0 around this fixture, identify any contradiction in the current assumptions, and state which remaining choices need implementation experiments rather than more abstract design.

It should not produce complexity merely because mature workflow/migration products contain it.
