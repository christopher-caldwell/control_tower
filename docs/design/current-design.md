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
- three-step-workspace.md
---

# Current design and decision audit

## Product identity

Control Tower is a personal, local, migration-style workbench for arbitrary user-owned executable actions. The developer owns the operations; Control Tower gives those operations an ordered up/down/verify lifecycle, a small amount of working context, visible results, and controls for moving through the work.

It is intentionally not an orchestration product. No login, hosting, scheduler, workers, DAG engine, built-in HTTP/database action model, or generalized automation platform is part of the current identity.

## Core transition model

The smallest model now has two useful pieces of runtime position:

- **completed step** — the last step whose up and verify completed successfully,
- **in-progress step** — the next step whose up succeeded but whose verify has not succeeded yet.

For a forward transition into step N:

~~~text
completed N-1
    |
    | N/up
    v
N in progress
    |
    | N/verify
    v
completed N
~~~

If N has no verify file, successful up completes N immediately.

The important point is that after verify failure both of these statements are true:

~~~text
completed:   N-1
in progress: N
~~~

There is no ambiguity and no need to pretend the external mutation never happened.

## Recovery from an in-progress step

If N/verify fails, the developer can inspect the effective context, change application code, and retry N/verify without rerunning N/up.

Or the developer can run N/down.

If N/down succeeds while N is in progress:

~~~text
completed:   N-1
in progress: none
~~~

Control Tower returns to the already-completed N-1 baseline. It does not require N-1/verify again as part of navigation. The author owns the correctness of N/down, just as the author owns a database down migration.

If N/down fails, N remains in progress and its context/results remain available.

## Normal backward movement

The same migration philosophy applies when the current step is fully completed.

From completed N back to completed N-1:

~~~text
N/down
  |
  | success
  v
completed N-1
~~~

No automatic target verification is required. A manual verify of N-1 can still be useful, but it is not part of the pointer-moving contract.

This keeps verify focused: it validates an **up transition before the target step becomes completed**.

## Minimal concepts

A workspace contains an ordered migration set and optional auxiliary actions.

A step is one migration unit with up/down and optionally verify.

A completed step is the last fully accepted step.

An in-progress step is the one forward target whose up has succeeded but whose verify has not yet succeeded.

Context carries small authored values between executions. While a step is in progress, the effective context must include values produced by that step's up so verify, inspect, and down can use them.

Auxiliary actions do not change migration bookkeeping.

A result is what Control Tower directly observes: executable, stdout, stderr, exit status, timing, and eventual machine-output data.

These concepts do not imply a generalized workflow execution model.

## Authoring boundary

User-owned executable files remain the core mechanism. A shebang can choose shell, Node, Python, or another installed interpreter; compiled executables fit the same boundary.

Control Tower does not understand SQL, HTTP, application business rules, or whether down truly reverses up. Future helpers may exist as sidecars, but they remain ordinary consumers of the executable/context contract.

## Leading filesystem probe

~~~text
workspace/
  steps/
    001-create-user/
      up
      down
      verify
    002-create-associated-record/
      up
      down
      verify
    003-mutate-record/
      up
      down
      verify
  actions/
    inspect
~~~

This convention still expresses the current lifecycle without central YAML/TOML. Config remains allowed to earn its way in later.

See [Three-step workspace design probe](three-step-workspace.md).

## Context direction

The process boundary still points toward:

~~~text
effective context -> executable
stdout/stderr -> human-visible result
machine output -> context changes
exit status -> operation result
~~~

The in-progress-step model now gives a concrete reason for a committed context plus working/pending changes. The exact encoding and persistence remain proposed in ADR-0003.

## Scope that has not earned its way in

No built-in drivers, mandatory SDK, expression language, dependency installer, scheduler, distributed execution, authentication, hosted service, automatic retry, universal rollback, or external-state certainty engine is selected.

The next design work is the **context lifecycle while a step is in progress**, especially how inspect/down consume working values and what happens when up itself exits nonzero after producing output.
