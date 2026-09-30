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
- ../history/2026-09-30-initial-design.md#e15-verify-before-step-commit
---

# Three-step workspace design probe

## Purpose

This is a concrete design exercise, not accepted filesystem syntax. Its job is to make abstractions earn their place.

The example models:

1. Create a test user.
2. Create an associated record in the state required by the ticket.
3. Invoke the mutation under development.

## Candidate workspace shape

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

The names and layout are illustrative. The shebang inside each executable chooses how it runs.

## The corrected step lifecycle

The target step owns both its mutation and its verification.

Moving from committed step 001 to step 002 is:

~~~text
001 committed
    |
    | 002/up
    v
002 pending
    |
    | 002/verify
    v
002 committed
~~~

The recorded step does **not** change to 002 until 002/verify succeeds.

This is the important correction to the previous departure-gate interpretation.

### Why this matters

002/up may create record_id. 002/verify likely needs record_id to check the result. Therefore the workbench needs a temporary/effective context for a pending transition even though 002 is not committed yet.

If verify fails:

~~~text
committed: 001
pending:   002

002/up:     passed
002/verify: failed

effective context:
  user_id: 123
  record_id: 456
~~~

The developer must be able to work with that state without pretending the transition committed.

## Step 1 — create user

001/up creates the fixture user and produces user_id.

001/verify validates the created-user state.

Only after both succeed does the recorded step become 001 and user_id become committed context.

## Step 2 — create associated record

002/up consumes user_id and produces record_id.

002/verify validates the associated-record state using the effective context, including record_id from the pending up.

Only after verify succeeds does the recorded step become 002 and record_id become committed context.

## Step 3 — mutate record

003/up consumes record_id and performs the operation under development.

003/verify validates the result.

This removes the earlier “final verify” ambiguity: if verify belongs to the transition into its own step, a full run to step 003 naturally executes 003/up then 003/verify before committing 003.

## Pending-transition ergonomics

This is now the central design question.

After 002/up succeeds and 002/verify fails, a useful UI might be:

~~~text
Committed: 001 Create User

Pending: 002 Create Associated Record
  up      PASSED
  verify  FAILED

[Verify Again]  [Inspect]  [Undo Pending]
~~~

The buttons are illustrative, but they expose real requirements.

### Verify Again

Reruns 002/verify using the same effective context. It must not rerun 002/up automatically, because a create-style up may not be idempotent.

If it passes, commit step 002 and its pending context.

### Inspect

Auxiliary inspection should probably receive the effective pending context; otherwise record_id would be unavailable exactly when it is most useful.

This is a proposal to test, not an accepted rule for every auxiliary action.

### Undo Pending

The natural candidate is to execute 002/down using the same effective context. If down succeeds, discard the pending context and return to the settled 001 state.

This is preferable to calling the operation “rollback,” because the author remains responsible for what down actually does.

If 002/down fails, the pending transition remains visible. Do not silently discard its context.

### Rerun Up

This should not be the default recovery action. Automatically rerunning 002/up after it already succeeded could create a duplicate fixture.

If an explicit “run up again” affordance ever exists, it should be clearly intentional. The first prototype may not need it at all while a pending transition exists.

## Forward full run

A run from 0 through 3 becomes:

~~~text
001/up
001/verify
commit 001

002/up
002/verify
commit 002

003/up
003/verify
commit 003
~~~

Each pair is serial. The tool stops on first failure.

This is migration-style behavior with a verification hook, not a DAG or orchestration engine.

## Backward walk remains to be tested

There are at least two plausible semantics for moving from committed 003 back to 002:

1. Run 003/down and commit 002 if down succeeds.
2. Run 003/down, then 002/verify, then commit 002.

The “verify target before changing the recorded step” principle suggests the second may be more consistent, but it can also make recovery awkward if the previous state is intentionally only partially restorable.

Do not settle this by symmetry alone. Test it against the example.

## Findings

### 1. Pending transition is real, but narrow

Earlier we tried to eliminate within-step state. Verification-before-commit proves that was too aggressive.

We need enough runtime state to remember:

- committed step,
- target/pending step,
- whether up succeeded,
- pending context changes,
- latest verify result.

That is still dramatically smaller than a general workflow execution model.

### 2. Context needs a committed and effective view

verify and pending inspection need the outputs from successful up before those outputs become committed context.

A conceptual model is:

~~~text
committed context
  + pending changes
  = effective context for pending step
~~~

Exact file formats and storage are still open.

### 3. Context removal still matters

A successful pending undo needs to discard values introduced by the pending up. Normal committed down transitions also need a way to remove identifiers that no longer belong to the resulting state.

### 4. Central config still has not earned its way in

Numeric ordering, fixed filenames, and shebangs still express this lifecycle.

### 5. Failure handling remains author-owned

A failed verify does not mean Control Tower knows how to repair the external system. It only knows the transition is not committed.

## Next design decision

Work through the pending-002 experience before choosing the output encoding:

1. 002/up succeeds and produces record_id.
2. 002/verify fails.
3. inspect the pending state.
4. change application code.
5. rerun verify and commit, **or** run 002/down and abandon the pending transition.

That exercise will tell us exactly what context lifecycle the process protocol must support.
