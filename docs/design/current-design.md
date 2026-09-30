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

Control Tower is a personal, local, migration-style workbench for arbitrary user-owned executable actions. A developer defines ordered steps, uses a UI to walk them forward or backward, runs individual actions while changing application code, and sees process results and shared context.

Rust is the chosen implementation language. Native UI versus a local browser UI is undecided.

The governing boundary is:

> You own the work. Control Tower runs it, records its position, passes small amounts of context, and gives you controls to move through it.

This is intentionally not an orchestration product. No login, hosted control plane, scheduler, worker model, DAG engine, built-in HTTP/database driver layer, or generalized automation platform is required.

## Core contract

The accepted direction is deliberately small:

1. Steps are ordered.
2. Step N can provide an up executable from N-1 toward N.
3. Step N can provide a down executable from N toward N-1.
4. Step N can provide a verify executable describing the checks that must pass before Control Tower records step N as current.
5. A forward change from N-1 to N is therefore: run N/up, then N/verify if present, then commit the recorded step change to N.
6. A failed up or verify does not commit the recorded step change.
7. The correctness of up, down, and verify is the author's responsibility, as with database migrations.
8. stdout/stderr and exit status are visible execution results.
9. Small shared context remains necessary because an up can create identifiers that verify and later steps need.

The tool records its own migration bookkeeping. It does not claim that external systems are transactional or automatically reversible.

## The newly exposed runtime concept: pending transition

Verification-before-commit means a successful up can have real side effects and produce useful context before the step is committed.

For example:

~~~text
recorded step: 001
active context:
  user_id: 123

run 002/up
  -> succeeds
  -> creates record_id: 456

run 002/verify
  -> fails
~~~

Control Tower cannot honestly call step 002 committed, but it also cannot throw away record_id if the developer needs it to inspect, retry verify, or run 002/down.

The smallest model therefore needs a **pending transition** in addition to the last committed step:

~~~text
committed step: 001
pending step: 002
pending up: succeeded
pending verify: failed
effective context:
  user_id: 123
  record_id: 456
~~~

This is not a workflow engine. It is the minimum bookkeeping implied by making verify part of the transition before commit.

The exact persistence and output protocol for pending context remains proposed in ADR-0003.

## Minimal concepts

A workspace contains an ordered migration set and optional auxiliary actions.

A step is one migration unit with up/down and optionally verify.

A committed step is the last step whose transition completed according to the workbench contract.

A pending transition is an in-progress move whose mutation has run but whose target step has not yet been committed.

Context is a small set of values carried between executions. While a transition is pending, the effective context may need to include both committed values and outputs produced by the pending up.

An auxiliary action runs without changing the committed step, such as inspect.

A result is what Control Tower directly observes: executable, stdout, stderr, exit status, timing, and eventual machine-output data.

These are conceptual responsibilities, not required Rust structs or database tables.

## Authoring boundary

User-owned executable files remain the core mechanism. A shebang can choose shell, Node, Python, or another installed interpreter; compiled executables fit the same boundary.

Control Tower does not understand SQL, HTTP, Node libraries, or business semantics. Future helpers can exist as sidecars that consume the same executable/context contract. They are not core execution primitives.

The [three-step design probe](three-step-workspace.md) continues to test whether a convention-only directory is sufficient before adding central configuration.

## Leading transition sequence

For a forward move into step N:

~~~text
committed N-1
   |
   | N/up
   v
pending N
   |
   | N/verify
   v
commit N
~~~

If N/up fails, the committed step remains N-1.

If N/up succeeds but N/verify fails, the committed step remains N-1 and N remains pending. The pending outputs need to stay usable for inspection and retry.

If N/verify later succeeds, Control Tower commits the context changes and the recorded step together as far as its local persistence model allows.

A step without verify can commit immediately after a successful up.

Backward transition verification is not yet specified by this correction. The current design should not silently infer a symmetric rule until we test the three-step example in reverse.

## Scope that has not earned its way in

No built-in database/HTTP action types, mandatory SDK, expression language, dependency installer, scheduler, distributed workers, DAG engine, authentication system, hosted service, automatic retries, universal rollback, or exact-once execution is selected.

The new pending-transition requirement does not justify a generalized workflow state machine. It only exists because verify is explicitly part of a step transition before the committed pointer changes.

## Current design probe

The active exercise remains [Three-step workspace design probe](three-step-workspace.md).

The next design work is now **pending-transition ergonomics**: what the UI allows after up succeeds but verify fails, what context those actions receive, and how down/abort should behave. The machine-output encoding should follow that decision rather than precede it.
