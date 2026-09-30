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
- ../history/2026-09-30-initial-design.md#e23-sqlite-moves-into-v0
- ../design/discovery-brief.md
- ../research/discovery-01.md
---

# ADR-0002: Use migration-style steps with verified completion

## Accepted core

Control Tower uses an ordered migration model. User-owned executables perform the mutations. Control Tower does not guarantee those mutations are semantically correct or reversible.

A directional transition becomes complete only after its mutation and any supplied directional verification succeed.

## Leading directional convention

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

The leading directional rule is:

~~~text
up   -> verify-up   -> complete higher step
down -> verify-down -> complete lower step
~~~

If the relevant verifier is absent, mutation exit 0 completes the transition.

The owner confirmed the four-role shape as the leading mechanism for formal discovery, explicitly subject to change when the real fixture is exercised. It is not an irreversible public contract.

## Active directional transition

After mutation success but before verification success, retain enough information to retry the matching verifier or invoke the supported reverse action without automatically repeating the mutation.

Earlier exploration illustrated that information as a completed checkpoint stack plus a transition containing source context, output patch, and candidate context. Those representations remain hypotheses, not accepted storage or public process-protocol requirements. The v0 proof only needs the UUID handoff and correct navigation results.

Do not rerun the mutation automatically when retrying its verifier. The author remains responsible for the verifier's behavior; calling a program a verifier is not a safety guarantee.

## Failed mutation

A nonzero mutation exit does not complete the directional transition. Record the attempt, do not automatically invoke its verifier, and stop the walk.

No richer recovery semantics are required for the first implementation. Special handling for a partially effective failed mutation—including whether to expose a matching down as an explicit cleanup action—is deliberately deferred until a working vertical slice demonstrates a concrete need.

## Session scope

SQLite is the v0 runtime adapter, as selected in [ADR-0004](0004-session-storage-port-and-adapters.md). Normal CLI exits retain the workbench state needed for subsequent invocations. Application behavior remains independent of the storage implementation.

An abnormal Rust-process termination has no v0 external-state recovery guarantee. This is distinct from a script or verifier returning failure while Control Tower remains able to record and report the result.

Changing stage-directory structure during a stored workbench run remains the author's responsibility; no detection or reconciliation is required. Multiple simultaneous instances are unsupported and require no project-specific protection.

## Consequence

The runtime remains local and serial. It needs enough bookkeeping for completed and unfinished directional work, but neither a general checkpoint/patch framework nor a crash-recovery engine is implied.

## Documentation correction during discovery

The source-review pass found obsolete memory-only session wording in this accepted record after SQLite had already been selected. That wording is corrected here to match the owner's later instruction, not to introduce a new storage decision. The first-discovery record preserves the finding and inspected baseline. The discovery closeout also moved mutation-failure recovery details out of the pre-implementation blocker set so the first vertical slice can be built before those edge cases are generalized.
