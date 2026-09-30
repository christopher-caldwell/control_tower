---
id: ADR-0003
title: Use session context checkpoints plus an active-transition patch
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
- ../history/2026-09-30-initial-design.md#e20-storage-is-an-adapter-concern
- ../decisions/0004-session-storage-port-and-adapters.md
- ../design/three-step-workspace.md
---

# ADR-0003: Use session context checkpoints plus an active-transition patch

## Standing

The strongest current session-state candidate is:

- one context checkpoint per completed step,
- one optional active directional transition,
- a small mutation-output patch used to build candidate context,
- stdout/stderr separate from machine state.

The state model belongs to application/core logic. **How that state is stored is an adapter concern covered by ADR-0004.**

## Storage-independent semantics

Do not define this ADR in terms of “a Vec in memory,” SQLite rows, or any other adapter representation.

The application needs logical state such as:

~~~text
completed checkpoints
optional active transition
source context
candidate context
recent execution result needed by current interaction
~~~

The application-owned storage port should expose whatever operations the capability actually needs. Concrete adapters decide how to represent those values.

## V0 memory behavior

The initial injected adapter is in-memory. Therefore session state disappears with the Rust process.

That is an adapter behavior, not a dependency of the transition algorithm.

A later SQLite adapter may preserve the same logical state beyond process lifetime without rewriting the up/down/verification logic.

## Forward context

From completed checkpoint C(N-1):

~~~text
N/up -> patch P
candidate C(N) = apply(C(N-1), P)
~~~

verify-up receives candidate C(N). On success it becomes the next completed checkpoint.

## Backward context

From completed C(N), the lower C(N-1) checkpoint is the target baseline:

~~~text
N/down -> optional patch D
candidate C(N-1) = apply(saved C(N-1), D)
~~~

verify-down can validate that candidate. On success, the application state settles at C(N-1).

Most down scripts need no Control Tower state output.

## Why patches still matter

A down may reestablish the lower logical state using replacement identifiers.

An override patch keeps checkpoint rewind ergonomic without forcing the adapter to understand application semantics.

## Source and candidate views

Directional verifiers may need both source and candidate context.

Exact process transport remains open.

## Mutation failure

If up/down exits nonzero:

- do not change completed state,
- do not automatically verify,
- show stdout/stderr/exit status,
- stop.

No v0 recovery/adoption state machine is required.

## Remaining decisions

- patch encoding,
- exact process representation of source/candidate context,
- whether auxiliary actions can mutate session context.

Persistence mechanism is **not** an open application-design question: memory is the v0 adapter and SQLite is the planned fast-follow adapter.
