---
id: ADR-0003
title: Use in-memory context checkpoints plus an active-transition patch
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-scope-reduction
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../history/2026-09-30-initial-design.md#e18-four-file-gauntlet
- ../history/2026-09-30-initial-design.md#e19-reject-unneeded-safety-machinery
- ../design/three-step-workspace.md
---

# ADR-0003: Use in-memory context checkpoints plus an active-transition patch

## Standing

The strongest current context candidate is deliberately ephemeral:

- in-memory context checkpoint per completed step,
- one optional active directional transition,
- a small mutation-output patch used to build the candidate context,
- stdout/stderr separate from machine state.

No Control Tower state persistence is required for v0.

## Process lifetime

The Rust process lifetime is the session lifetime.

On normal restart or crash, Control Tower begins again with:

~~~text
completed: 0
context: {}
active transition: none
~~~

There is no state file, migration-history database, resume protocol, or crash reconciliation.

External side effects from previously executed scripts remain outside Control Tower's responsibility.

## Forward context

From completed checkpoint C(N-1):

~~~text
N/up -> patch P

candidate C(N) = apply(C(N-1), P)
~~~

verify-up receives the candidate context. If it succeeds, candidate C(N) becomes the next in-memory completed checkpoint.

## Backward context

From completed checkpoint C(N), the session already has C(N-1):

~~~text
N/down -> optional patch D

candidate C(N-1) = apply(saved C(N-1), D)
~~~

verify-down validates that candidate. On success, pop C(N) and settle the candidate as current C(N-1).

Most down scripts need no Control Tower output.

## Why patches still matter

A down may recreate an equivalent previous state using new identifiers.

Example:

~~~text
saved C02:
  record_id: 456

03/down creates:
  record_id: 789
~~~

An override patch lets candidate C02 use 789.

The checkpoint handles ordinary rewind; the patch handles exceptional differences.

## Source and candidate views

Directional verifiers may need:

- source context,
- candidate context.

verify-down may need an old identifier to prove it was deleted even though that value should not appear in the candidate lower-step context.

Exact process transport remains open.

## Mutation failure

If up/down exits nonzero:

- do not change completed state,
- do not automatically verify,
- show stdout/stderr/exit status,
- stop.

Any machine output from the failed mutation can be shown with the attempt, but v0 does not need an adoption/recovery mechanism.

If the Rust process is restarted, all of this session information disappears.

## No persistence-derived machinery

The following are explicitly unnecessary for v0:

- persistence backend,
- crash recovery,
- durable active transitions,
- structural identity,
- migration drift checks,
- file/content checksums,
- multi-instance locking.

These can only return if a real personal-workflow problem justifies them.

## Remaining decisions

- patch encoding,
- whether auxiliary actions can mutate context,
- exact process representation of source and candidate context.
