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

Control Tower remains a personal, local, migration-style workbench for arbitrary user-owned executable actions. The developer owns the work. Control Tower owns ordered navigation, process execution, visible results, and a small session-local context model.

It is intentionally not an orchestration product.

## Core model

The strongest current step shape remains:

~~~text
step/
  up
  down
  verify-up     # optional
  verify-down   # optional
~~~

A directional mutation changes the completed step only after its optional directional verifier succeeds.

~~~text
forward:  up   -> verify-up   -> complete higher step
backward: down -> verify-down -> complete lower step
~~~

The runtime needs only:

- in-memory completed-step context checkpoints for this session,
- at most one active directional transition,
- recent execution results for the current session.

The author remains responsible for the semantics of every executable.

## Session lifetime is the Rust process lifetime

For the initial product, Control Tower state is **not durable**.

If the Rust process exits or crashes:

~~~text
completed checkpoints -> gone
active transition     -> gone
session context       -> gone
~~~

The next launch starts from position 0 with empty Control Tower context.

External side effects caused by user scripts remain whatever they are. Control Tower does not attempt to discover, reconcile, recover, or undo them after restart.

This is an intentional scope decision, not a missing reliability feature.

## No concurrency machinery

This project is for one local user and one intended instance.

No locks, leases, multi-writer protection, or race-prevention system is needed.

Running two instances against the same workspace is unsupported and the user's responsibility.

## No structural-drift protection

Control Tower does not need to protect the user from changing step directories during a session.

Renaming, inserting, removing, or reordering steps while state exists is unsupported. If the author changes the structure and wants a clean model, restart Control Tower and begin again.

No migration identity database, directory checksum, structural reconciliation, or adoption flow is required.

Editing the executable contents themselves during a session remains a normal intended workflow.

## In-memory context checkpoints

Context checkpoints remain useful **within the current session**.

Example:

~~~text
completed 01:
  user_id: 123

completed 02:
  user_id: 123
  record_id: 456
~~~

They are not persisted snapshots of the outside world. They are just enough bookkeeping to make up/down navigation ergonomic while the app is running.

A forward transition starts from the current checkpoint. Its mutation output patch builds a candidate context. verify-up sees that candidate. On success, the candidate becomes the next in-memory checkpoint.

A backward transition starts with the saved lower-step checkpoint as its target baseline. Any down output can patch that baseline. verify-down sees the candidate lower-step context before the completed pointer changes.

## Source and candidate context

A directional verifier may need both sides of a transition.

Example: down deletes record_id=456. verify-down may need 456 to prove it no longer exists even though the candidate lower-step context correctly omits it.

The process contract therefore has two useful conceptual views:

- **source context** — context before the directional mutation,
- **candidate context** — context that would become current if verification succeeds.

Exact transport remains open.

## Active transition

An active transition is session-local bookkeeping:

~~~text
step: 03
direction: up | down
source context
mutation patch
candidate context
latest results
~~~

If verify-up or verify-down fails, the transition remains available in memory for inspection and verifier retry without rerunning the mutation.

If the Rust process dies, this is discarded. The next run starts over.

## Failure policy

### Mutation exits nonzero

Do not change the completed step and do not run the verifier automatically. Show stdout/stderr/exit status and stop.

Any external side effects are the author's responsibility.

### Mutation succeeds; verifier fails

Keep the active transition in memory. Allow verifier retry and inspection without rerunning the mutation.

### Rust process exits/crashes

Discard Control Tower state. Start fresh on next launch. No recovery protocol.

## Deferred escape hatch

A future workspace-level reset executable could provide a user-authored “back everything out / get me to a known beginning” escape hatch.

That is intentionally **not** part of the current contract and should not be designed until the normal up/down lifecycle proves insufficient.

## What still has not earned scope

No scheduler, authentication, hosting, DAG, built-in drivers, retries, durability layer, concurrency control, structural-drift protection, transaction emulation, external snapshots, or expression language is required.

The remaining design work is small:

1. whether verify-down becomes the default optional convention,
2. exact context patch/output encoding,
3. whether auxiliary actions can change Control Tower context.

See [Three-step workspace design probe](three-step-workspace.md).
