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
- ../history/2026-09-30-initial-design.md#e18-four-file-gauntlet
- ../design/three-step-workspace.md
---

# ADR-0002: Use migration-style steps with verified completion

## Accepted core

Control Tower uses an ordered migration model. User-owned executables perform the mutations. Control Tower does not guarantee those mutations are semantically correct or reversible.

Forward completion already requires the target step's optional verification before the completed pointer advances.

## Directional-verification recommendation

The four-role design survived the current gauntlet:

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

The recommended directional rule is:

~~~text
up   -> verify-up   -> complete higher step
down -> verify-down -> complete lower step
~~~

If the relevant verifier is absent, mutation exit 0 completes the transition.

The owner has confirmed this four-role shape as the **leading mechanism for formal discovery**, explicitly subject to change as the real fixture is exercised. It is not being treated as an irreversible public contract.

## Active directional transition

After mutation success but before verification success, keep one active transition:

~~~text
step
direction: up | down
source context
mutation output patch
candidate context
latest mutation/verifier result
~~~

Do not rerun the mutation automatically while that transition exists.

Retrying the verifier is safe from Control Tower's perspective because the author explicitly supplied a verifier; its external side effects remain the author's responsibility.

## Failed mutation

A nonzero mutation exit does not advance or create a normal successful mutation transition.

Record the attempt and stop.

This is deliberately stricter than trying to infer success from partial external effects. Recovery after a broken mutation can be added later without contaminating the normal state model.

## Session scope

The completed stack and active transition are in-memory session bookkeeping.

A Control Tower restart starts over from zero. No durable migration history, structural-drift detection, or crash recovery is required.

Changing step structure mid-session is unsupported and left to the author.

Multiple simultaneous Control Tower instances for the same workspace are also unsupported; no locking is needed.

## Consequence

The runtime remains serial and small: in-memory completed stack + one optional active transition.

No DAG, scheduler, retries, distributed workers, persistence layer, or external transaction engine is implied.
