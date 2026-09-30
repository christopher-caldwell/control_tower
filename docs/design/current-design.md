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

> You own the work. Control Tower runs it, records its own position, passes small amounts of context, and gives you controls to move through it.

This is intentionally not Dagu-lite and not an orchestration product. No login, hosted control plane, scheduler, worker model, DAG engine, built-in HTTP/database driver layer, or generalized automation platform is required.

## Core contract

The accepted direction is deliberately small:

1. Steps are ordered.
2. Step N up moves from recorded position N-1 to N.
3. Step N down moves from recorded position N to N-1.
4. Multi-step movement executes those files sequentially and stops on first nonzero exit.
5. Successful up records the new position; failed up/down does not move the pointer.
6. If the current step has verify, it must exit successfully immediately before any forward transition to the next step.
7. A failed verify leaves the pointer and current context intact and blocks only the forward transition.
8. A future/previous verify result is not treated as a durable permission token; requesting Next should run the gate again.
9. The correctness of up, down, and verify is the author's responsibility, as with database migrations.
10. stdout/stderr and exit status are visible execution results.
11. Small shared context remains a leading candidate because generated IDs must survive across steps; the exact machine-output protocol is still open.

The pointer records which authored migration transitions succeeded. It does not claim that external systems have been proven correct.

## Minimal concepts

A workspace contains an ordered migration set and optional auxiliary actions.

A step is one migration unit with up/down and optionally verify.

A position is Control Tower's recorded migration index. Position 0 means no step has been applied in the current workspace/session.

verify belongs to the current position and gates forward movement out of it.

An auxiliary action runs without changing the pointer, such as inspect. Control Tower does not enforce that it is read-only.

Context is a small set of values carried between executions, such as user_id and record_id.

A result is what Control Tower directly observes: executable, stdout, stderr, exit status, timing, and eventual machine-output data.

No separate controller-level “stage state machine” is required. The UI can retain the latest verify result for feedback without treating it as durable truth.

## Authoring boundary

User-owned executable files remain the core mechanism. A shebang can choose shell, Node, Python, or another installed interpreter; compiled executables fit the same boundary.

Control Tower does not understand SQL, HTTP, Node libraries, or business semantics. Future helpers can exist as sidecars that consume the same executable/context contract. They are not core execution primitives.

The [three-step design probe](three-step-workspace.md) suggests that a central configuration file has not yet earned its place: numeric directories and fixed executable names express the smallest example.

## Leading context and I/O direction

~~~text
Control Tower context -> process input view
executable stdout/stderr -> human-visible result
executable machine output -> context updates
~~~

The verification gate now simplifies successful transition timing:

~~~text
step N up succeeds
  -> current context updates
  -> recorded position N

later, before N -> N+1:
  step N verify runs
  -> pass: allow next up
  -> fail: stay at N
~~~

The concrete example only requires scalar identifiers and removal when walking down. Rich structured context still has not earned scope.

See [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## Dagu lesson

Dagu remains useful inspiration for execution results and output-passing patterns. The owner's local trial clarified the product boundary: Dagu solves a broader workflow/orchestration problem and felt substantially heavier than this intended workbench.

Control Tower intentionally optimizes for walking a user-authored migration set during development.

## Scope that has not earned its way in

No built-in database/HTTP action types, mandatory SDK, expression language, dependency installer, scheduler, distributed workers, DAG engine, authentication system, hosted service, automatic retries, universal rollback, exact-once execution, or rich external-state model is selected.

The UI can simply show the recorded position, the attempted action, the latest verify result, and stdout/stderr.

## Current design probe

The active exercise remains [Three-step workspace design probe](three-step-workspace.md): create a user, create an associated record, and mutate that record.

Verification timing is now settled. The next meaningful design work is the smallest context publication contract, especially what happens to machine output emitted by a process that later exits nonzero.
