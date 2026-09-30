---
id: CT-DESIGN
title: Current design and decision audit
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../history/2026-09-30-initial-design.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- three-step-workspace.md
---

# Current design and decision audit

## Product identity

Control Tower remains a personal, local, migration-style workbench for arbitrary user-owned executable actions. The developer owns the work. Control Tower owns ordered navigation, execution bookkeeping, visible results, and a small context protocol.

It is intentionally not an orchestration product.

## Core model after the gauntlet

The four-role step survives as the strongest current design:

~~~text
step/
  up
  down
  verify-up     # optional
  verify-down   # optional
~~~

A directional mutation changes the completed step only after its optional directional verifier succeeds.

~~~text
forward:  up   -> verify-up   -> commit higher step
backward: down -> verify-down -> commit lower step
~~~

The runtime needs only:

- the stack of completed steps and their context checkpoints,
- at most one active directional transition,
- recent execution results.

The author remains responsible for the semantics of every executable.

## Context checkpoints

The strongest result of the design gauntlet is a **context checkpoint per completed step**.

Example:

~~~text
step 01 checkpoint:
  user_id: 123

step 02 checkpoint:
  user_id: 123
  record_id: 456
~~~

This is Control Tower metadata only. It is not a snapshot of the external database or API.

A forward transition starts from the current completed checkpoint. Its mutation output becomes a transition-local patch, producing a candidate target context. verify-up sees that candidate. On success, the candidate is pushed as the next completed checkpoint.

A backward transition starts from the current completed checkpoint but has an existing lower-step checkpoint as its target baseline. Its down output can optionally patch that baseline. verify-down sees the candidate lower-step context before the completed pointer changes.

This solves the ordinary stale-ID problem without forcing every down script to publish unsets.

## Why down still needs optional context output

Pure checkpoint restoration is not general enough.

Suppose step 03/down returns to the logical conditions of step 02 by **creating a replacement record** rather than restoring the exact old record. The old step-02 checkpoint may contain record_id=456, but down produces a valid replacement record_id=789.

Therefore the candidate lower-step context is:

~~~text
saved target checkpoint
+
down output patch
=
candidate target context
~~~

If down emits no context changes, ordinary checkpoint restoration is automatic.

This keeps common down authoring minimal while preserving arbitrary user-owned behavior.

## Source and candidate context

A directional verifier can need information from both sides of the transition.

Example: down deletes record_id=456. verify-down may need the old ID to prove it no longer exists, even though the candidate lower-step context correctly omits it.

Therefore the process contract should eventually expose conceptually:

- **source context** — context before the directional mutation,
- **candidate context** — context that would become current if verification succeeds.

Exact environment variable/file names remain open, but both views have now earned their way into the design.

## Active transition

An active transition is narrow bookkeeping:

~~~text
step: 03
direction: up | down
phase: mutation | verification
source checkpoint
mutation output patch
candidate context
latest results
~~~

If verify-up or verify-down fails, the transition remains active and can be inspected or reverified without rerunning the mutation.

This is not generalized workflow state; it is the minimum needed to avoid repeating non-idempotent mutations.

## Failure rules that survived

### Mutation exits nonzero

Do not change the completed step. Do not automatically run its verifier. Record stdout/stderr/exit result and stop.

The mutation may have caused external side effects. That remains the author's recovery problem.

### Mutation succeeds; verifier fails

Keep an active transition. Retain its source, output patch, and candidate context. Allow verifier retry without rerunning the mutation.

### Down verifier fails

Keep the original completed pointer and the active down transition. The external world may already resemble the lower step; Control Tower simply has not accepted the transition.

### Tool/process interruption

Persist transition intent before launching a mutation so a restart can tell that execution was interrupted. Do not automatically retry an interrupted mutation.

A future recovery action may allow the developer to verify or repair an interrupted transition, but automatic inference is not required for the initial model.

## Structural drift

Persisted migration state must be tied to the ordered step structure.

Renaming, removing, inserting, or reordering step directories below the completed position can invalidate the meaning of saved checkpoints.

Control Tower should detect structural drift and stop automatic navigation until the developer explicitly resets/adopts a new structure.

Do **not** checksum executable contents as a hard gate. Editing verify/up/down scripts during development is part of the intended workflow. Database migration systems often validate migration identity/checksums; Control Tower should borrow structural-drift detection without making normal script editing hostile.

## Concurrency

One workspace should have a single writer. A local file lock or storage-level lock is enough.

This is not distributed coordination; it simply prevents two Control Tower instances from mutating the same pointer/checkpoint stack concurrently.

## What still has not earned scope

No scheduler, authentication, hosted control plane, DAG, built-in action drivers, retries, transaction emulation, external snapshots, expression language, or helper ecosystem is required for the core.

The remaining hard design edges are small:

1. exact mutation-output patch encoding,
2. how much recovery UI to expose after a mutation itself fails/interruption,
3. whether auxiliary actions can mutate Control Tower context,
4. concrete persistence backend.

See [Three-step workspace design probe](three-step-workspace.md) for the gauntlet cases.
