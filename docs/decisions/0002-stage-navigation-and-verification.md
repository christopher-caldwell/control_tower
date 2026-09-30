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
- ../history/2026-09-30-initial-design.md#e17-directional-verification-proposal
- ../design/three-step-workspace.md
---

# ADR-0002: Use migration-style steps with verified completion

## Accepted decision

Control Tower uses an ordered migration model.

A forward step N becomes completed only after N/up and its optional forward verification succeed.

The runtime can therefore record a last completed step plus one active/in-progress transition. The author owns the semantic correctness of all executables.

## Current directional-verification proposal

The owner has proposed giving each step four possible executables:

~~~text
up
down
verify-up
verify-down
~~~

with both verifiers optional.

This extension is under evaluation and is not yet promoted to a separate accepted decision.

It would make the transition rule symmetric:

~~~text
forward:  N/up   -> N/verify-up   -> commit N
backward: N/down -> N/verify-down -> commit N-1
~~~

If the relevant verifier is absent, a successful mutation completes the directional transition immediately.

## Why it fits the core boundary

verify-up and verify-down remain ordinary executables. Control Tower only cares about their exit status and output.

The split avoids requiring one generic verifier to infer which direction just ran or to describe the entire state of the world.

## Active transition model

If the directional verifier fails after the mutation succeeds, the completed pointer does not move.

A compact active-transition record is enough:

~~~text
step: N
direction: up | down
mutation: passed
verification: failed
working context: ...
~~~

For an up failure, the previous completed step remains current.

For a down verification failure, the source completed step remains current until the down transition is accepted.

This does not mean the external world is unchanged. It is only Control Tower's migration bookkeeping.

## Recovery

A failed verify-up can be retried without rerunning up.

A failed verify-down can be retried without rerunning down.

Backing out a failed verify-up naturally uses the same step's down operation. Under the four-file proposal, verify-down can validate that cleanup before the active transition is cleared.

Do not automatically retry directional mutations; they may not be idempotent.

## Context consequence

Directional verification means context needed to check a mutation must remain available until its verifier completes.

In particular, down verification may need source-step identifiers even when the external objects have already been removed.

This strengthens the case for retaining completed context snapshots or an equivalent context-history mechanism. The exact context policy remains ADR-0003's concern.

## What remains accepted regardless of this proposal

The core stays serial, local, user-authored, and migration-style.

No DAG, scheduler, hosted control plane, automatic retry engine, transaction emulation, or universal rollback guarantee follows from adding a second optional verifier.
