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

This is intentionally **not Dagu-lite** and not an orchestration product. No login, hosted control plane, scheduler, worker model, DAG engine, built-in HTTP/database driver layer, or generalized automation platform is required for the current product identity.

## The problem

A development ticket often needs a disposable fixture in a sequence of useful conditions: create a test user, create associated data, mutate it, inspect it, change application code, restore an earlier useful condition, and run the mutation again.

Today that recipe can be scattered among SQL snippets, curl commands, scripts, copied IDs, and remembered ordering. Control Tower is meant to remove the coordination friction without taking ownership of the underlying operations.

A representative loop is:

~~~text
0 -> create fixture -> add required data -> run mutation
                                           |
                                      inspect result
                                           |
                                  change application code
                                           |
                       down to useful point -> run mutation again
~~~

Running one step and walking the whole sequence are both first-class behaviors.

## Core contract

The accepted direction is deliberately small:

1. Steps are ordered.
2. Step N can provide an up executable that moves from recorded position N-1 to N.
3. Step N can provide a down executable that moves from recorded position N to N-1.
4. Moving several positions executes the required up or down files sequentially and stops on the first nonzero exit.
5. The current position is bookkeeping owned by Control Tower. It is not a claim that the outside world has been proven to match.
6. The author is responsible for making up, down, and any assertion executable correct. This is the same responsibility boundary expected from a database migration author.
7. stdout, stderr, exit status, and the attempted step are visible results of execution.
8. Assertions/checks remain part of the direction, but their exact timing relative to changing the recorded position is still unresolved.
9. A small shared context between executables remains a leading candidate because generated IDs must survive across steps. The exact wire and storage protocol is still open.

If an up or down file exits nonzero, Control Tower stops and does not change the recorded position for that transition. The executable may already have caused external side effects. Control Tower reports the failure; it does not invent a rollback or maintain a second inferred model of the external system.

## Minimal concepts

A **workspace** contains an ordered migration set and any auxiliary actions.

A **step** is one ordered migration unit. Up belongs to the move into that step; down belongs to the move out of it toward the previous position.

A **position** is Control Tower's recorded migration index: 0 through N. Position 0 means no step has been applied in the current workspace/session.

An **auxiliary action** is an executable that can be run without changing the recorded position, such as an inspection script. Control Tower does not attempt to infer whether an auxiliary action is truly read-only.

A **context** is the small set of authored values carried between executions, such as user_id or record_id. Context is distinct from the recorded position.

A **result** is what Control Tower can know directly about an invocation: command, stdout, stderr, exit status, timing, and any machine-output payload defined by the eventual protocol.

These are conceptual responsibilities, not required Rust structs or database tables.

## Authoring boundary

User-owned executable files remain the core authoring mechanism. A shebang can choose shell, Node, Python, or another installed interpreter. A compiled executable also fits the same process boundary.

Control Tower does not need to understand SQL, HTTP, Node libraries, application repositories, or business semantics. Future helpers such as HTTP, Postgres, or TypeScript conveniences are allowed to exist as sidecars that consume the same executable/context contract. They are not core execution primitives and do not need consideration for the first version.

The exact filesystem convention is being tested in [Three-step workspace design probe](three-step-workspace.md). One important result is that a central configuration file has not yet earned its place: numeric directories and fixed executable names are enough to express the smallest example. This does not reject configuration; it makes metadata prove why it is needed.

## Leading context and I/O direction

The existing leading candidate still separates three things:

~~~text
Control Tower context -> process input view
executable stdout/stderr -> human-visible result
executable machine output -> context updates
~~~

This separation avoids turning stdout into a brittle control protocol. Environment variables remain useful transport, especially for paths to context/output files, but they are not themselves durable storage.

The three-step probe narrows the immediate need. The concrete example only requires scalar identifiers and the ability to remove an identifier when moving down. Nested objects, multiline values, type systems, expression languages, and many variable scopes have not earned implementation scope.

See [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## What the Dagu trial established

Dagu remains useful inspiration, especially for execution results, per-step visibility, and output-passing patterns. A local trial also clarified the product boundary: Dagu is a broader workflow/orchestration system with server state, authentication setup, workflow management, scheduling and other platform concerns. The owner found it close in capability but substantially heavier than the desired workbench.

That is not a criticism of Dagu and does not prove a market gap. It establishes that Control Tower is intentionally optimizing for a different interaction: walking a user-authored migration set during development.

See [Existing tools](../research/existing-tools.md).

## Scope that has not earned its way in

No built-in database/HTTP action types, mandatory SDK, expression language, dependency installer, scheduler, distributed workers, generalized DAG engine, authentication system, hosted service, built-in AI agent, automatic retries, universal rollback, or exact-once execution is selected.

No complex “external state certainty” model is selected either. The UI can say that recorded position 1 remains current and that attempt 1 -> 2 failed. That is enough to communicate what Control Tower actually knows.

A managed helper ecosystem may be useful later, but the core must remain able to run ordinary user-owned executables with no helper dependency.

## Current design probe

The active concrete exercise is [Three-step workspace design probe](three-step-workspace.md): create a user, create an associated record, and mutate that record. It is intentionally small enough to reveal whether an abstraction is actually necessary.

The main unresolved issue exposed by that example is assertion timing. That question should be resolved before the context commit/failure protocol because it determines when a generated identifier must become visible and when a position is considered advanced.
