---
id: ADR-0002
title: Use migration-style ordered transitions verified before commit
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
- ../history/2026-09-30-initial-design.md#e15-verify-before-step-commit
- ../design/three-step-workspace.md
---

# ADR-0002: Use migration-style ordered transitions verified before commit

## Decision

Control Tower uses an ordered migration model rather than a generalized workflow graph.

For a forward transition into step N:

~~~text
committed N-1
   |
   | N/up
   v
pending N
   |
   | N/verify, if present
   v
commit N
~~~

The recorded step changes to N **only after** N/up succeeds and N/verify succeeds when verify exists.

If N/up fails, the recorded step remains N-1.

If N/up succeeds but N/verify fails, the recorded step still remains N-1, while the workbench retains a pending N transition so the developer can inspect it, retry verification, or use an authored recovery/down action.

The correctness of up, down, and verify is the author's responsibility. Control Tower runs the contract; it does not guarantee semantic correctness or transactional behavior in the external system.

## Why verification is part of the target transition

The directory:

~~~text
002-create-associated-record/
  up
  down
  verify
~~~

describes the move into the useful state represented by step 002.

up attempts to establish it. verify checks it. Only then is the step committed by the workbench.

This matches the explicit requirement that verify run before the step change is recorded.

## Pending transition is required bookkeeping

Because up can succeed before verify fails, the runtime needs a narrow pending-transition concept.

It must retain enough information to:

- rerun target verify without rerunning up,
- expose outputs from up to verify and inspection,
- allow the author to undo/abandon the pending transition,
- commit the step and context if verify later succeeds.

This does not imply retries, DAG scheduling, worker state, or a generalized orchestration execution model.

## Pointer and external state

The committed pointer records completed Control Tower transitions, not external truth.

A failed verify can leave real external side effects from up. The workbench does not move the pointer, but it also must not pretend nothing happened. The pending transition and its results are sufficient evidence for the developer to decide what to do next.

## Down semantics

down remains author-owned compensation rather than guaranteed undo.

The exact rule for verification during a normal backward transition is still open. Do not infer it solely from the forward rule. The three-step probe will test whether the previous step's verify should run before a backward pointer change.

For a **pending forward transition**, however, using the pending target's down as an explicit “undo pending” operation is the leading candidate because that up already ran.

## Full run

A forward full run repeats:

~~~text
N/up
N/verify
commit N
~~~

for each step, stopping on first failure.

Because verify belongs to the target transition, the final step is verified before the full run can record it as complete. There is no special end-of-run verification rule needed.

## Consequences

The model is slightly more stateful than a plain migration pointer because verification occurs after mutation but before commit.

That complexity has earned its way in: without retaining pending outputs, a failed verify could make it impossible to inspect or undo a newly created fixture without rediscovering its identifiers.

The next design work is not to generalize this state machine. It is to make the pending-transition ergonomics small and obvious.
