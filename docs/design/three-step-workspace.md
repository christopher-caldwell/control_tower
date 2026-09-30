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
- ../history/2026-09-30-initial-design.md#e17-directional-verification-proposal
---

# Three-step workspace design probe

## Four-file candidate

The current probe gives each step up to four executables:

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

up and down are the mutations. verify-up and verify-down are optional directional checks.

The exact filenames are not accepted syntax yet.

## Forward lifecycle

From completed 02 toward 03:

~~~text
03/up
  |
  | success
  v
03 / up direction in progress
  |
  | 03/verify-up
  v
completed 03
~~~

If verify-up fails:

~~~text
completed: 02
active transition:
  step: 03
  direction: up
  mutation: passed
  verify: failed
~~~

The developer can inspect, change application code, and rerun verify-up without rerunning up.

To abandon the in-progress forward transition, 03/down is the natural authored operation. If verify-down exists, it can validate that reversal before clearing the active transition and returning to the completed-02 baseline.

This is slightly stronger than the previous “down success immediately clears pending 03” rule and is exactly what the four-file proposal is intended to test.

## Backward lifecycle from a completed step

From completed 03 toward 02:

~~~text
03/down
  |
  | success
  v
03 / down direction in progress
  |
  | 03/verify-down
  v
completed 02
~~~

If verify-down fails:

~~~text
completed: 03
active transition:
  step: 03
  direction: down
  mutation: passed
  verify: failed
~~~

The pointer remains on the last fully completed state until the directional transition verifies.

This mirrors the forward rule without requiring verify-up and verify-down to assert the same conditions.

## Why not reuse 02/verify-up after 03/down?

The operation being validated is 03/down, so the author who wrote 03/down is best positioned to define what counts as its success.

03/verify-down can also use source-step identifiers that 02 would not naturally own.

This avoids making every stage verifier a universal description of the entire external state.

## Optional means genuinely optional

A step can have:

~~~text
up only
~~~

and complete forward when up exits zero.

Or:

~~~text
up
verify-up
~~~

for verified forward completion.

Or the full:

~~~text
up
down
verify-up
verify-down
~~~

for checked movement in both directions.

Control Tower should not require placeholder files.

Whether down itself is optional remains a separate authoring question; a missing down simply means normal backward navigation through that step is unavailable.

## Failure ergonomics

The interesting cases are now symmetric.

### Forward verification fails

~~~text
02 completed
03/up succeeds
03/verify-up fails

options:
  Inspect
  Verify Up Again
  Down
~~~

If Down is chosen and verify-down exists:

~~~text
03/down
03/verify-down
=> return to completed 02
~~~

### Backward verification fails

~~~text
03 completed
03/down succeeds
03/verify-down fails

options:
  Inspect
  Verify Down Again
~~~

A possible “Up to restore 03” escape hatch is symmetric, but it has not earned UI scope yet. The author can always choose recovery behavior once we test a real scenario.

## Context insight

Directional down verification changes the context question.

Suppose completed 02 includes record_id and 02/down deletes that record. 02/verify-down may still need record_id to verify the record is gone. Therefore the workbench should keep the source/effective context available while the down transition is in progress.

Only after verify-down succeeds should the workbench settle context for completed 01.

This makes per-completed-step context snapshots attractive:

~~~text
completed 01 snapshot:
  user_id

completed 02 snapshot:
  user_id
  record_id
~~~

A verified 02/down could restore the stored 01 snapshot automatically.

That would remove a common burden from down authoring: down would not need to tell Control Tower to unset record_id merely because it deleted the associated external record.

This is a design candidate, not yet a decision. It should be compared with explicit down output mutations.

## What the four-file model earns

It gives up and down equal treatment without adding a controller assertion language.

It also produces a stable rule:

> A directional mutation changes the completed pointer only after its optional directional verifier succeeds.

That rule is easy to explain and test.

## Next exercise

Use step 02 to compare the two context strategies:

1. **snapshot restore:** 02/down + 02/verify-down succeeds, then Control Tower restores the saved step-01 context;
2. **explicit output:** 02/down publishes whatever set/unset operations are needed for the step-01 context.

The better option should minimize authoring work without making Control Tower infer application semantics.
