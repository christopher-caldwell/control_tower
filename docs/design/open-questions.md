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

The four-role directional transition model survived the current design gauntlet. The context candidate has narrowed to completed checkpoints plus an active-transition patch.

## Q1 — Accept directional verify-down?

No tested case exposed a problem with optional verify-down. It improves symmetry and lets the author validate the exact operation that ran.

The proposed rule is:

~~~text
down -> verify-down -> commit lower step
~~~

with verify-down optional.

**Exit criterion:** owner accepts/rejects the four-role step as the default convention.

## Q2 — What patch encoding is smallest?

The context model now needs:

- set scalar values,
- occasionally unset inherited values,
- preserve normal stdout/stderr,
- parse deterministically.

Compare a small line protocol against a small JSON change document.

Checkpoint rewind means down scripts usually need no state output, which should keep this protocol small.

## Q3 — How much recovery UI after mutation failure/interruption?

Normal behavior is clear: stop and keep the completed pointer.

Decide whether first version needs an explicit “verify/adopt interrupted mutation” recovery path or whether inspection/manual cleanup is sufficient.

Avoid automatic retry.

## Q4 — Can auxiliary actions mutate Control Tower context?

Read-only inspection is easy.

Allowing arbitrary actions to change context raises the question of which completed checkpoint or active candidate they mutate.

Keep auxiliary context output disabled until a concrete workflow requires it.

## Q5 — Persistence backend

The minimum durable state is small:

- ordered structural identity,
- completed step/checkpoint stack,
- optional active transition,
- recent result metadata.

Compare one atomic local state file with SQLite on implementation ergonomics.

## Q6 — Structural drift UX

Define the smallest behavior when step directories are inserted, removed, renamed, or reordered around persisted state.

Do not block normal edits to executable contents.

## Validation order

Settle Q1, then choose the patch encoding against one shell up and one Node up/down. Recovery and persistence can follow without changing the user-facing migration contract.
