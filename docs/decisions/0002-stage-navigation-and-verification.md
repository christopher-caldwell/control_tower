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

verify-down remains technically a proposal until the owner accepts it, but no gauntlet case exposed a reason to collapse the two verifiers back into one.

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

## Structural drift

Persisted completed-step identity must be compared with the discovered ordered step structure.

Block automatic navigation if the migration structure changed incompatibly.

Do not make executable-content checksums a hard validity condition. Editing scripts during active development is expected.

## Consequence

The runtime remains serial and small: completed stack + one optional active transition.

No DAG, scheduler, retries, distributed workers, or external transaction engine is implied.
