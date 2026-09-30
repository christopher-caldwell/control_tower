---
id: ADR-0003
title: Use completed context checkpoints plus an active-transition patch
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-gauntlet-refinement
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../history/2026-09-30-initial-design.md#e18-four-file-gauntlet
- ../research/existing-tools.md
- ../design/three-step-workspace.md
---

# ADR-0003: Use completed context checkpoints plus an active-transition patch

## Standing

The original “one mutable state bag” idea has been narrowed.

The strongest current candidate is:

- one context checkpoint for each completed step on the current migration stack,
- one optional active directional transition,
- a small mutation-output patch used to build that transition's candidate context,
- stdout/stderr kept separate from machine state.

This survived the design gauntlet better than pure snapshot restoration or fully explicit down bookkeeping.

## Forward context

From completed checkpoint C(N-1):

~~~text
N/up -> patch P

candidate C(N) = apply(C(N-1), P)
~~~

verify-up receives the candidate context. If it succeeds, push candidate C(N) as the completed checkpoint.

## Backward context

From completed checkpoint C(N), the saved target checkpoint C(N-1) is already available.

~~~text
N/down -> optional patch D

candidate C(N-1) = apply(saved C(N-1), D)
~~~

verify-down validates that candidate. On success, pop C(N) and settle the candidate as the current C(N-1).

Most down scripts need no Control Tower context output at all. Their normal context rewind is automatic.

## Why down patches remain necessary

The previous logical step may be reestablished with different identifiers.

Example:

~~~text
old C02:
  record_id: 456

03/down creates replacement:
  record_id: 789
~~~

Restoring 456 would be stale. Down can publish an override patch with 789.

This is the escape hatch that makes checkpoints general enough for arbitrary user-owned operations.

## Source and candidate views

A verifier may need both.

verify-down often needs a source identifier to prove an object was deleted even though that identifier correctly does not belong in the candidate lower-step context.

The future process protocol should therefore expose conceptually:

- source context,
- candidate context.

Exact file names/env variables remain open.

## Mutation patch semantics

The first examples still primarily need scalar set operations. Checkpoint rewind removes much of the earlier need for down to publish explicit unsets.

An unset operation is still useful when a forward mutation intentionally removes an inherited key or a down override needs to remove something from the target checkpoint.

The exact encoding remains open.

## Failed mutation output

If a mutation exits nonzero after emitting machine output, preserve that output with the failed attempt as recovery evidence. Do not automatically merge it into a completed checkpoint.

Whether an explicit recovery/adoption operation later promotes those values is outside the normal transition contract.

This is the current recommendation from the gauntlet; it remains subject to owner approval.

## Durability

Persist active-transition intent before launching mutation work.

Persist mutation result/patch before starting its verifier.

This lets a restart distinguish:

- no transition,
- mutation interrupted,
- mutation succeeded and verifier pending/failed.

The storage backend is still open.

## Structural identity

Context checkpoints are meaningful only relative to the ordered migration structure.

Persist enough structural identity to detect renames/reorders/removals around completed steps.

Do not hard-lock executable contents: changing scripts while debugging is an intended use case.

## Remaining decisions

- patch encoding,
- storage backend,
- whether auxiliary actions can mutate context,
- recovery UX for interrupted/nonzero mutation,
- retention of old attempt history after checkpoints are popped.
