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

The design is intentionally session-local. Durability, race protection, and migration-structure safety are not open questions for v0; they are out of scope.

## Q1 — Accept directional verify-down?

The current candidate is:

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

The gauntlet did not expose a problem with verify-down, and it keeps directional checks simple.

**Exit criterion:** owner accepts/rejects the four-role step as the default convention.

## Q2 — What patch encoding is smallest?

The in-memory context model needs:

- set scalar values,
- occasionally unset inherited values,
- preserve normal stdout/stderr,
- parse deterministically.

Compare a tiny line protocol against a small JSON patch document.

## Q3 — Can auxiliary actions mutate Control Tower context?

Read-only inspection is simple.

Allowing arbitrary actions to change context raises the question of which checkpoint or candidate they mutate.

Keep auxiliary context output disabled unless a concrete workflow requires it.

## Q4 — How are source and candidate context exposed?

Directional verifiers can need both.

Choose the smallest language-neutral transport after the patch format is clear.

## Explicitly out of scope for v0

- persistent session state,
- crash recovery,
- structural drift detection,
- migration history/checksums,
- multi-instance locking,
- automatic external-state reconciliation.

A future workspace-level reset executable is a deferred idea, not a current design task.

## Validation order

Settle Q1, then choose the patch encoding against one shell action and one Node action. Source/candidate transport should follow from that.
