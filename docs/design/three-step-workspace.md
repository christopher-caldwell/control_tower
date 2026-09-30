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
- ../history/2026-09-30-initial-design.md#e13-three-step-design-probe
- ../history/2026-09-30-initial-design.md#e16-completed-and-in-progress-steps
---

# Three-step workspace design probe

## Candidate workspace

~~~text
workspace/
  steps/
    001-create-user/
      up
      down
      verify

    002-create-associated-record/
      up
      down
      verify

    003-mutate-record/
      up
      down
      verify

  actions/
    inspect
~~~

This remains illustrative rather than accepted filesystem syntax.

## Happy-path sequence

The user described the intended sequence as:

~~~text
01/up

02/up
02/verify

03/up
03/verify -> FAIL
~~~

After 02/verify passes:

~~~text
completed: 02
in progress: none
~~~

After 03/up succeeds but 03/verify fails:

~~~text
completed:   02
in progress: 03
~~~

That is the central model.

02 did not become “not done” merely because 03 failed verification. 03 also did not disappear merely because it was not completed. The workbench can represent both facts directly.

## Step lifecycle

A step can be understood with a very small lifecycle:

~~~text
not started
    |
    | up succeeds
    v
in progress
    |
    | verify succeeds
    v
completed
~~~

verify failure keeps the step in progress.

If no verify file exists, successful up can move directly to completed.

Failures are recorded as execution results rather than additional permanent domain states.

## Recovery after 03 verify failure

The developer has two main paths.

### Keep working on 03

~~~text
completed:   02
in progress: 03

[Inspect]
[Verify 03 Again]
~~~

The effective context includes outputs produced by 03/up. The developer can change application code and rerun only 03/verify.

If verify later passes:

~~~text
completed:   03
in progress: none
~~~

### Back out 03

Run:

~~~text
03/down
~~~

using the effective in-progress context.

If it succeeds:

~~~text
completed:   02
in progress: none
~~~

No 02/verify is required to make that bookkeeping change. 02 was already completed before 03 started, and the author has defined 03/down as the operation that returns to that baseline.

If 03/down fails, keep 03 in progress and retain its working context/results.

## Why this is useful ergonomically

A failed verify should not force the developer to:

- rerun 03/up,
- rediscover IDs produced by 03/up,
- reconstruct 02,
- or convince Control Tower about external truth.

The workbench only needs to preserve enough information to continue working with 03 or back it out.

A plausible UI is:

~~~text
✓ 001 Create User
✓ 002 Create Associated Record
◐ 003 Mutate Record
    up      passed
    verify  failed

[Verify Again] [Inspect] [Down to 002]
~~~

The exact visuals are not decided, but this is the interaction being optimized.

## Backward movement from a completed step

If 03 had completed and the developer later wants to return to 02:

~~~text
03/down
  -> success
  -> completed becomes 02
~~~

Again, automatic 02/verify is not required. verify belongs to completing an up transition, not proving a down transition.

The author can manually run 02/verify if useful.

## Context consequence

The in-progress model implies an effective context:

~~~text
committed context from completed 02
+
working changes from 03/up
=
effective context for 03/verify, inspect, and 03/down
~~~

If 03 verifies successfully, the working changes become committed.

If 03/down succeeds while 03 is in progress, the working changes can be abandoned/removed as part of returning to the 02 baseline, subject to the eventual context protocol.

The exact mechanics still need testing.

## What remains open

- What exact context changes are retained from successful up while verify is failing?
- What happens to machine output if up itself exits nonzero?
- Does down publish context changes, or can an in-progress down simply discard that step's working delta?
- How should context move backward from a **completed** step where its outputs were already committed?
- Does verify remain read-only from the workbench-context perspective?
- Does convention-only authoring survive a real project?

## Next exercise

Use 03 as a deliberately failing step and model these two loops:

~~~text
02 completed
03/up succeeds
03/verify fails
change app code
03/verify succeeds
=> 03 completed
~~~

and:

~~~text
02 completed
03/up succeeds
03/verify fails
03/down succeeds
=> 02 completed
~~~

Those two paths should drive the initial context-storage contract.
