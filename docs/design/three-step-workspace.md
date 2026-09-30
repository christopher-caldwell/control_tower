---
id: CT-DESIGN-THREE-STEP
title: Three-step workspace design probe
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- current-design.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../history/2026-09-30-initial-design.md#e19-reject-unneeded-safety-machinery
---

# Three-step workspace design probe

## Candidate authoring shape

~~~text
workspace/
  steps/
    001-create-user/
      up
      down
      verify-up
      verify-down

    002-create-associated-record/
      up
      down
      verify-up
      verify-down

    003-mutate-record/
      up
      down
      verify-up
      verify-down

  actions/
    inspect
~~~

Both verifiers are optional. The filenames remain illustrative.

## Session-local runtime model

Within one Rust process session, completed steps can keep context checkpoints:

~~~text
0  {}
1  { user_id }
2  { user_id, record_id }
~~~

At most one directional transition can be active.

Nothing in this model survives a Control Tower restart in v0.

### Forward

~~~text
source checkpoint 02
    |
    | 03/up
    v
candidate 03 context
    |
    | 03/verify-up
    v
completed checkpoint 03
~~~

### Backward

~~~text
source checkpoint 03
    |
    | 03/down
    v
candidate 02 context
    |
    | 03/verify-down
    v
completed checkpoint 02
~~~

## Candidate context

Forward:

~~~text
candidate_N = checkpoint_(N-1) + up_patch_N
~~~

Backward:

~~~text
candidate_(N-1) = saved_checkpoint_(N-1) + down_patch_N
~~~

The down patch is optional. It exists for cases where down reconstructs an equivalent previous state with different identifiers.

## Gauntlet results that still matter

### Happy path

~~~text
01/up
01/verify-up
complete 01

02/up
02/verify-up
complete 02
~~~

Clean.

### verify-up fails

~~~text
02 completed
03/up succeeds
03/verify-up fails
~~~

Keep 02 completed and 03 active in memory. Allow Inspect and Verify Up Again without rerunning up.

Clean.

### Back out an in-progress up

~~~text
03/down
03/verify-down
~~~

If both succeed, clear 03 active state and return to completed 02.

Clean.

### Completed down

~~~text
03 completed
03/down
03/verify-down
=> 02 completed
~~~

Use the saved session checkpoint for 02 plus any down patch.

Clean.

### verify-down fails

Keep completed 03 plus the active down transition. Allow inspection and Verify Down Again without rerunning down.

Clean.

### Down recreates prior logical state with new IDs

Historical 02 checkpoint has record_id=456, but 03/down creates replacement 789.

Down patch overrides record_id to 789 before verify-down.

Pure checkpoint restoration fails here; checkpoint + patch works.

### Higher step overwrites an existing key

Checkpoint stack restores the earlier value automatically on a normal down unless down explicitly overrides it.

Clean.

### Mutation exits nonzero after partial external work

Do not advance. Show the failure. Whatever happened externally is the author's problem.

No adoption/recovery state machine is required for v0.

Accepted limitation.

### Rust process crashes

All Control Tower session state is lost. On restart:

~~~text
completed: 0
context: {}
active transition: none
~~~

External systems are untouched.

Accepted limitation.

### Step directories change during a session

Unsupported. Restart and start over if the workspace structure changed.

No detection or reconciliation.

Accepted limitation.

### Two Control Tower instances

Unsupported. No locking.

Accepted limitation.

### Verifier absent

Mutation exit 0 completes that directional transition.

Clean.

### Down absent

Backward navigation through that step is unavailable.

Clean.

## What was removed after the gauntlet

The following ideas were initially surfaced as robustness improvements but are intentionally out of scope:

- durable transition state,
- crash recovery,
- migration-structure identity/checks,
- structural-drift detection,
- single-writer locking,
- persistence backend selection.

They solve problems this personal workbench does not need to solve right now.

## Deferred reset idea

A future root/workspace reset executable could be an explicit author-owned escape hatch:

~~~text
workspace/
  reset
  steps/
  actions/
~~~

Its possible meaning would be “attempt to return the external test environment to this workspace's beginning.”

No semantics are being designed now. This is only retained as a future idea.

## Remaining pressure tests

- exact mutation patch/output encoding,
- whether auxiliary actions may publish context,
- whether verify-down should be adopted as the default optional convention.
