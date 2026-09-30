---
id: ADR-0002
title: Use migration-style ordered up/down navigation
type: decision
status: accepted
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: explicit-user-direction
decision_date: '2026-09-30'
sources:
- ../history/2026-09-30-initial-design.md#e03-migration-like-navigation
- ../history/2026-09-30-initial-design.md#e04-assertion-gates
- ../history/2026-09-30-initial-design.md#e12-core-identity-settled
- ../design/three-step-workspace.md
---

# ADR-0002: Use migration-style ordered up/down navigation

## Decision

Control Tower uses an ordered migration model rather than a generalized workflow graph.

Step N moves between recorded positions N-1 and N:

~~~text
position 0 <-- down 1 -- position 1 <-- down 2 -- position 2
           ---  up 1 -->            ---  up 2 -->
~~~

Moving forward several positions runs the necessary up executables in order. Moving backward runs the necessary down executables in reverse order. Execution stops on the first nonzero exit.

If a transition executable succeeds, Control Tower may move its recorded pointer. If it fails, Control Tower does not move the pointer for that transition and shows the failure.

The correctness of an up or down operation is the author's responsibility. A down action is not guaranteed undo, and Control Tower does not inspect application semantics to determine whether an external system actually matches the recorded pointer. This is intentionally analogous to a database migration runner executing author-written migrations.

Assertions/checks are part of the desired workbench, but the exact timing of an assertion relative to moving the recorded pointer is not decided by this ADR. That narrower question is documented in [the three-step probe](../design/three-step-workspace.md#4-assertions-expose-the-first-real-semantic-choice).

## Why this decision

The development loop is inherently sequential: establish fixture state, make a mutation, inspect, change code, move back, and repeat. A migration chain gives forward and backward navigation without requiring pairwise reset definitions or a dependency graph.

It also preserves the central product boundary. Control Tower runs user-authored operations; it does not orchestrate distributed work or claim transactional guarantees it cannot provide.

## Failure semantics

A failed command can have external side effects before returning nonzero. Control Tower does not need a second “unknown external state” engine to represent this.

The UI should communicate only what the tool actually knows, for example:

~~~text
Recorded position: 1
Attempted: 1 -> 2
Exit: 1
~~~

The developer is responsible for deciding the next operation.

This replaces the earlier proposal to model a richer “uncertain fixture state.” That proposal was technically defensible but inconsistent with the deliberately dumb migration-runner boundary later chosen by the owner.

## Down semantics

down N is the author-provided operation used when walking from recorded position N to N-1. It may be a true inverse, a compensating API call, a reset script, or any other operation the author considers suitable.

If down N is absent, the normal backward path through that step is unavailable. Control Tower should not pretend the transition happened by changing only its pointer.

No generic rebuild/reset guarantee is part of this decision. A future convenience for starting over must still reduce to authored operations plus ordinary pointer/context management.

## Full runs and individual runs

Both are first-class product goals.

A full forward walk executes sequential up operations until the target/end or the first failure. Individual execution lets the developer work one transition at a time while editing application code between attempts.

Exact UI labels and whether “run all” means from position 0 or “continue to end from the current position” remain presentation/interaction questions, not reasons to add an orchestration model.

## Assertions remain intentionally narrow

The earlier discussion accepted the idea of assertions as gates, but not their exact lifecycle. The three-step example shows that arrival verification and departure gating have different context/failure consequences.

Do not add an assertion DSL. The smallest direction is another user-owned executable where exit status expresses pass/fail. Resolve when it runs before implementing richer semantics.

## Consequences

The core can remain serial and understandable. There is no need for DAG scheduling, dependency resolution, distributed execution, retries, or transaction emulation.

The cost is explicit: Control Tower may record position 1 while the external system has been partly changed by a failed attempt toward position 2. That is accepted as part of the author-responsibility boundary rather than hidden behind a false guarantee.
