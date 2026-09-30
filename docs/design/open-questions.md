---
id: CT-QUESTIONS
title: Open questions and validation
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- current-design.md
- three-step-workspace.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
---

# Open questions and validation

This is a decision queue, not a release plan.

## Settled: completed plus in-progress step

If N/up succeeds but N/verify fails:

~~~text
completed:   N-1
in progress: N
~~~

N/verify can be retried without rerunning up.

N/down can back out the in-progress step. On successful down, clear N-in-progress and remain at completed N-1.

Backward navigation does not automatically reverify N-1.

See [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).

## Q1 — How should context move backward from a completed step?

This is now the most useful unresolved storage question.

When completed 02 is moved down to 01, values introduced by 02 may need to disappear or revert.

Compare:

- requiring 02/down to explicitly publish context set/unset changes,
- retaining a per-step context delta/snapshot so Control Tower can restore the prior completed context after successful down.

**Exit criterion:** a create-record step can go 01 -> 02 -> 01 without stale record_id and without making down authoring unnecessarily awkward.

## Q2 — What happens to machine output when up itself fails?

If up emits an ID and later exits nonzero, does that value become usable recovery context or only captured attempt output?

Do not conflate this with successful up followed by failed verify.

## Q3 — What exact machine-output format is smallest?

Once backward context is understood, compare a tiny set/unset text protocol against a small JSON change document.

The actual requirements are still primarily scalar IDs and removal.

## Q4 — Is verify read-only from the context perspective?

No current example requires verify to publish values.

Prefer read-only unless a concrete use case demonstrates otherwise.

## Q5 — Does convention-only authoring remain sufficient?

Numeric directories plus up/down/verify still express the lifecycle. Keep config out until a real metadata need appears.

## Validation order

Model context through 01 -> 02 -> 03-in-progress -> 02 and 02 -> 01. That should settle backward context ownership before choosing the output encoding.

These are design steps, not implementation milestones.
