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

Control Tower is a personal, local, migration-style workbench for arbitrary user-owned executable actions. The developer owns the operations; Control Tower provides ordered navigation, process execution, visible results, and a small amount of working context.

It is intentionally not an orchestration product. No login, hosting, scheduler, workers, DAG engine, built-in HTTP/database action model, or generalized automation platform is part of the current identity.

## Core migration model

The accepted core remains:

- ordered steps,
- author-owned up and down executables,
- optional verification before a forward step becomes completed,
- one last completed step,
- a narrow in-progress transition when a mutation has run but verification has not completed,
- stop on failure rather than inventing rollback guarantees.

The author owns semantic correctness, just as with database migrations.

## Four-executable step candidate

The current design candidate is deliberately symmetric:

~~~text
003-mutate-record/
  up
  down
  verify-up
  verify-down
~~~

Both verification executables are optional.

The naming is illustrative; the four responsibilities are the important part.

### Forward

~~~text
completed 02
    |
    | 03/up
    v
03 up transition in progress
    |
    | 03/verify-up
    v
completed 03
~~~

If verify-up is absent, successful up completes 03 immediately.

### Backward

~~~text
completed 03
    |
    | 03/down
    v
03 down transition in progress
    |
    | 03/verify-down
    v
completed 02
~~~

If verify-down is absent, successful down completes the move to 02 immediately.

This avoids overloading one verifier with two potentially different contracts. verify-up asks whether up established the intended forward state. verify-down asks whether down established the intended backward state.

The owner has proposed this symmetric model; verify-down remains under evaluation rather than recorded as a finalized decision.

## Runtime bookkeeping implication

“In progress” can no longer mean only “the next step is moving upward.”

A more general conceptual model is:

~~~text
last completed step
+
optional active transition:
  step
  direction: up | down
  mutation result
  verification result
  working context
~~~

For the original failure example:

~~~text
completed: 02
active:
  step: 03
  direction: up
  up: passed
  verify-up: failed
~~~

For a downward verification failure from completed 03:

~~~text
completed: 03
active:
  step: 03
  direction: down
  down: passed
  verify-down: failed
~~~

This is still narrow migration bookkeeping, not a workflow engine.

## Why directional verification is attractive

Up and down do not necessarily produce mirror-image states.

For example, 03/up might close a record while 03/down reopens it. verify-up can assert “closed”; verify-down can assert “open.” A single generic verify file would need to infer direction or implement both meanings itself.

Separating the checks keeps each executable dumb and independently runnable.

It also preserves the author-responsibility boundary: Control Tower only uses exit status; it does not interpret what “open” or “closed” means.

## Context implication

Directional verification makes a context-snapshot approach more interesting.

During a forward transition, verify-up may need values created by up.

During a backward transition, verify-down may still need values from the completed source step to prove they were removed or changed externally.

Therefore Control Tower should not eagerly delete context merely because down returned zero. Context should remain available through verify-down and only settle after the whole directional transition succeeds.

One candidate is to keep a context snapshot for each completed position and restore the prior snapshot after a verified down. Another is to require down to publish explicit context changes. That choice remains open in ADR-0003.

## Authoring boundary

All four files are ordinary user-owned executables. A shebang chooses the runtime. Control Tower does not understand SQL, HTTP, Node, or the meaning of the assertions.

Future helpers may exist as sidecars but are not core primitives.

## Scope still excluded

No scheduler, authentication, hosting, DAG execution, automatic retry, distributed state, built-in drivers, universal rollback, or external-state certainty model has earned scope.

The next design exercise is to run the three-step example in both directions with optional verify-up/verify-down and determine which context model makes down authoring simplest.
