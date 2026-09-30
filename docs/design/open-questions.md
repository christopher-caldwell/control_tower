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
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
---

# Open questions and validation

The v0 storage implementation is memory, but storage is already architecturally isolated behind an application-owned port so SQLite can follow without rewriting transition logic.

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

The session context model needs:

- set scalar values,
- occasionally unset inherited values,
- preserve normal stdout/stderr,
- parse deterministically.

Compare a tiny line protocol against a small JSON patch document.

## Q3 — How are source and candidate context exposed?

Directional verifiers can need both.

Choose the smallest language-neutral transport after the patch format is clear.

## Q4 — Can auxiliary actions mutate Control Tower context?

Read-only inspection is simple.

Allowing arbitrary actions to change context raises the question of which checkpoint or candidate they mutate.

Keep auxiliary context output disabled unless a concrete workflow requires it.

## Not an open architecture question

The v0 entry boundary is settled:

- CLI is the only driving adapter,
- the CLI invokes application use cases rather than containing business logic,
- the executable entry point is the composition root,
- future Tauri/HTTP entry layers are additional adapters over the same application layer,
- no generic frontend/transport abstraction is required now.

The storage boundary is also settled:

- application/core owns the state port,
- memory adapter is v0,
- SQLite adapter is a fast follow,
- entry point chooses and injects the adapter.

Exact port method signatures should be derived from implementation use cases; do not predesign a generic CRUD repository.

## Explicitly out of scope for v0

- SQLite implementation,
- durable crash recovery,
- structural drift detection,
- migration history/checksums,
- multi-instance locking,
- automatic external-state reconciliation.

A future workspace-level reset executable remains a deferred idea.

## Validation order

Settle Q1, then choose the patch encoding against one shell action and one Node action. Source/candidate transport should follow from that.
