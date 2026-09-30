---
id: ADR-0002
title: Use migration-style ordered up/down navigation with forward verification gates
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
- ../history/2026-09-30-initial-design.md#e14-verification-gates-before-advancing
- ../design/three-step-workspace.md
---

# ADR-0002: Use migration-style ordered up/down navigation with forward verification gates

## Decision

Control Tower uses an ordered migration model rather than a generalized workflow graph.

Step N moves between recorded positions N-1 and N:

~~~text
position 0 <-- down 1 -- position 1 <-- down 2 -- position 2
           ---  up 1 -->            ---  up 2 -->
~~~

Moving forward several positions runs the necessary up executables in order. Moving backward runs the necessary down executables in reverse order. Execution stops on the first nonzero exit.

If an up executable succeeds, Control Tower records the new position and applies whatever context publication policy ADR-0003 eventually defines. If the transition executable fails, Control Tower does not move the pointer for that transition.

The correctness of an up or down operation is the author's responsibility. A down action is not guaranteed undo, and Control Tower does not inspect application semantics to determine whether an external system actually matches the recorded pointer. This is intentionally analogous to a database migration runner executing author-written migrations.

## Verification gate

If the current step defines a verify executable, that executable **must run successfully before Control Tower advances forward to the next step**.

For example:

~~~text
position 1
  |
  | step 2 up
  v
position 2
  |
  | step 2 verify   <-- mandatory before forward advance
  |
  | step 3 up
  v
position 3
~~~

This means step 2 up is responsible for entering position 2. Step 2 verify is responsible for gating departure from position 2 toward position 3.

A successful up therefore does not wait for verify before recording its position. This is important ergonomically: if verify fails, the developer remains at position 2 with the context produced by step 2 available for inspection and repair.

A verify failure does not move the position. The developer can inspect, change application code, run verify again, or move backward using the authored down path.

If a step has no verify executable, there is no verification gate for that forward transition.

## Verification state is intentionally small

Control Tower does not need a second persisted state machine for “entered / ready / blocked.”

The useful bookkeeping is:

- current recorded position,
- the latest verify attempt and result for display,
- the next requested transition.

When the user asks to advance, Control Tower should run the current step's verify immediately before the next up when a verify file exists. A previously successful manual verify is feedback, not a permanent permission token.

This avoids treating stale verification as authoritative while keeping the machinery dumb.

Whether a final target/end position is automatically verified when there is no subsequent forward transition remains an interaction question rather than part of this decision.

## Why this decision

The development loop is inherently sequential: establish fixture state, inspect it, make a mutation, change code, move back, and repeat. A migration chain gives forward and backward navigation without pairwise reset definitions or a dependency graph.

The verification gate matches the user's original example:

~~~text
0 -> 1 -> 2 [must meet x,y] -> 3
~~~

It also preserves the product boundary. Control Tower runs user-authored operations and checks; it does not interpret what they mean.

## Failure semantics

A failed command can have external side effects before returning nonzero. Control Tower does not need a second “unknown external state” engine to represent this.

The UI should communicate only what the tool directly knows, for example:

~~~text
Recorded position: 2
Attempted verify for step 2
Exit: 1
Forward advance: blocked
~~~

The developer decides the next operation.

## Down semantics

down N is the author-provided operation used when walking from recorded position N to N-1. It may be a true inverse, a compensating API call, a reset script, or any operation the author considers suitable.

If down N is absent, the normal backward path through that step is unavailable. Control Tower should not pretend the transition happened by changing only its pointer.

Verification is a forward gate. The leading interaction model does not require current-step verification before moving backward, because a failed verify is exactly when backward movement may be needed. This is an implementation/UX interpretation of the accepted gate rule and can be revisited if a real example requires otherwise.

## Full runs and individual runs

Both are first-class goals.

A forward walk repeats:

~~~text
up into N
record N
verify N before leaving N
up into N+1
~~~

Individual execution lets the developer remain at N, manually run verify as often as useful, inspect results, change application code, and only then request the next step.

Exact UI labels and final-position verification remain presentation questions, not reasons to add orchestration machinery.

## Consequences

The core remains serial and understandable. There is no need for DAG scheduling, dependency resolution, distributed execution, retries, transaction emulation, or a controller-owned assertion DSL.

The cost is explicit: Control Tower may record position 2 while step 2 verify fails. That is not contradictory. The pointer records which authored up/down migrations have succeeded; verify determines whether the developer may advance forward from that position.
