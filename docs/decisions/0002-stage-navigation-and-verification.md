---
id: ADR-0002
title: Use migration-style steps with verified completion
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
- ../history/2026-09-30-initial-design.md#e16-completed-and-in-progress-steps
- ../design/three-step-workspace.md
---

# ADR-0002: Use migration-style steps with verified completion

## Decision

Control Tower uses an ordered migration model.

A forward step N is complete only after:

~~~text
N/up
N/verify, if present
~~~

both succeed.

The workbench therefore tracks:

- the last **completed** step,
- and optionally the next **in-progress** step.

For example, if 03/up succeeds and 03/verify fails:

~~~text
completed:   02
in progress: 03
~~~

Both facts are meaningful and sufficient.

## Forward semantics

From completed N-1:

1. Run N/up.
2. If up fails, N does not become in progress and N-1 remains completed.
3. If up succeeds, N becomes in progress and its working outputs are available.
4. Run N/verify when present.
5. If verify succeeds, N becomes completed and the in-progress marker clears.
6. If verify fails, N remains in progress and N-1 remains the last completed step.

A step without verify completes immediately after successful up.

The author owns the semantic correctness of all executables.

## Working within an in-progress step

The developer must be able to operate on N without rerunning N/up:

- inspect using N's effective context,
- change application code,
- rerun N/verify,
- run N/down to back out N.

The runner should not automatically rerun N/up after it has succeeded.

## Down from an in-progress step

If N is in progress because up succeeded but verify did not:

~~~text
N/down
~~~

is the authored route back to completed N-1.

If down succeeds, clear the in-progress step and return to the existing N-1 completed baseline.

If down fails, keep N in progress and preserve its results/context for further inspection or recovery.

No automatic N-1/verify is required. The correctness of N/down is the author's responsibility.

## Down from a completed step

The same principle applies to ordinary backward movement:

~~~text
completed N
  |
  | N/down
  v
completed N-1
~~~

A successful down changes the completed pointer to N-1. Control Tower does not require N-1/verify as part of backward navigation.

verify is specifically the gate that lets an up transition become **completed**.

Manual verification of the resulting step can still be exposed as a useful action.

## Why this model

It mirrors the developer's actual mental model:

- 02 can be done,
- 03 can be underway,
- failed 03 verification does not erase 02,
- 03/down backs out the underway work.

This is enough statefulness to make iteration comfortable without creating a workflow/orchestration engine.

## Full run

A forward full run repeats for each step:

~~~text
up
verify
complete
~~~

and stops on the first failure.

If verify fails, the run stops with the previous step completed and the target step in progress.

## Consequences

The runtime needs a narrow in-progress record and working context.

It does not need generalized retries, dependency resolution, distributed state, or an “external truth” model.

Context lifecycle for in-progress and completed down transitions is intentionally delegated to ADR-0003 and the three-step probe.
