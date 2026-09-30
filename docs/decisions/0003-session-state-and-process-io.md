---
id: ADR-0003
title: Keep v0 state handoff minimal and storage-independent
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: discovery-leading-candidate
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../history/2026-09-30-initial-design.md#e22-first-formal-discovery-seed
- ../decisions/0004-session-storage-port-and-adapters.md
- ../design/discovery-brief.md
---

# ADR-0003: Keep v0 state handoff minimal and storage-independent

## Standing

Earlier exploration considered context checkpoints, source/candidate views, and mutation patches.

Those remain useful hypotheses, but they are **not v0 requirements**.

The first formal discovery fixture has one concrete handoff requirement: stage 1 creates an opaque UUID and later stages/verifiers need access to that same UUID.

Discovery should implement the smallest mechanism that supports that requirement and only generalize when a concrete use case forces it.

## Architectural boundary

Whatever state representation discovery chooses, application/core behavior must remain independent of the concrete storage adapter.

ADR-0004 governs that boundary:

- application/core owns the semantic state port,
- SQLite is the v0 runtime adapter,
- memory may be used as a test/fake adapter.

This ADR concerns the logical data that needs to flow, not how the adapter stores it.

## Initial proof requirement

The fixture needs a token such as:

~~~text
workspace/run identifier:
  550e8400-e29b-41d4-a716-446655440000
~~~

Stage 1 creates a file named by that UUID.

Stages 2 and 3 and their directional verifiers need to operate on that same file.

That is sufficient for the first discovery round.

## Do not predesign a generic context system

Do not require, before evidence:

- arbitrary nested values,
- typed key/value storage,
- merge/patch languages,
- multiple variable scopes,
- source/candidate files as public contract,
- expression interpolation,
- an SDK.

Discovery may use an internal state object or temporary protocol as needed. Any mechanism that proves broadly useful should be documented afterward with the concrete reason.

## Directional-transition implication

If a mutation succeeds and verification fails, Control Tower may need to remember the UUID while the stage remains in progress so verification can be retried without rerunning the mutation.

That is a genuine behavioral need.

How it is represented is an implementation/discovery question.

## CLI/process-lifetime implication

This tension is now resolved at the product level.

SQLite is part of v0 so one-shot CLI invocations can share the minimal workbench state required by the UUID fixture and directional-transition lifecycle.

Discovery should still keep the logical handoff minimal; SQLite availability is not a reason to invent a generalized context system.

## Remaining question

After the concrete UUID fixture works end-to-end, revisit whether the observed implementation warrants a richer context contract.

Until then, keep the product requirement at “pass the identifier needed by later stages.”
