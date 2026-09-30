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
- ../history/2026-09-30-initial-design.md#e14-verification-gates-before-advancing
---

# Three-step workspace design probe

## Purpose

This is a concrete design exercise, not accepted filesystem syntax. Its job is to make abstractions earn their place.

The example models:

1. Create a test user.
2. Create an associated record in the state required by the ticket.
3. Invoke the mutation under development.

The developer can inspect at any point, change application code outside Control Tower, move down to an earlier position, and run forward again.

## Candidate workspace shape

The smallest useful filesystem may need no central configuration file:

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

A convention-first layout is worth testing before YAML/TOML. A config file may later earn its place for metadata that the filesystem cannot express cleanly.

## Position and context

| Recorded position | Meaning in this example | Context that may be useful |
| --- | --- | --- |
| 0 | Nothing applied by Control Tower | empty |
| 1 | create-user up succeeded | user_id |
| 2 | create-associated-record up succeeded | user_id, record_id |
| 3 | mutate-record up succeeded | user_id, record_id |

The position is migration bookkeeping, not a proof about external state.

### Step 1 — create user

up creates the fixture user and publishes user_id.

down removes that user and should remove user_id from carried context.

verify checks whatever the author requires before allowing forward movement from position 1.

### Step 2 — create associated record

up uses user_id, creates the required associated data, and publishes record_id.

down removes or compensates for that record and should remove record_id from context.

verify is a mandatory forward gate when present. It can check the conditions that must hold before step 3 is allowed to run.

### Step 3 — mutate record

up uses record_id to invoke the operation under development.

down is the author's chosen compensation: reopen the record, reset fields, call a project-specific repair script, or anything else suitable.

verify can be run manually to inspect the final position. Whether a full “run to end” automatically runs the final verify is still open because there is no subsequent forward transition to gate.

### Auxiliary inspect action

actions/inspect reads the current context, queries whatever systems the author wants, and prints useful state to stdout. It does not change the recorded position.

Control Tower does not enforce that inspect is read-only.

## Walking forward

From position 0 to position 3:

~~~text
step 1 up
  success -> position 1

step 1 verify
  success -> step 2 may run

step 2 up
  success -> position 2

step 2 verify
  success -> step 3 may run

step 3 up
  success -> position 3
~~~

The key semantic is now settled: **verify belongs to the current position and gates the next forward transition.**

This creates a useful interactive pause inside a step:

~~~text
position 2
  |
  +-- inspect
  +-- verify -> fail
  +-- change application code
  +-- verify -> pass
  +-- next
       |
       +-- step 2 verify runs again
       +-- step 3 up
~~~

Running verify manually is useful feedback. Requesting Next should run verify again immediately before advancing rather than treating an old pass as a durable permission token.

## Walking backward

From position 3 back to position 1:

~~~text
step 3 down
  success -> position 2

step 2 down
  success -> position 1
~~~

The current leading interaction does not require verify before down. A failing verify should not trap the user in the current position.

Any nonzero down exit stops the walk. A missing down file means the normal backward path is unavailable.

## What this example teaches us

### 1. “Stage” does not need to be a second domain object

The position plus the ordered step is enough. “Stage” can remain conversational UI language if useful, but the core model does not need separate stage and step entities.

### 2. Central configuration has not earned its way in

Order comes from numeric prefixes. Direction comes from up/down names. Runtime comes from shebangs.

This is not a ban on config. The first config field should solve a demonstrated problem.

### 3. Context needs set and remove

Forward execution needs user_id and record_id. Backward execution needs to discard identifiers that no longer belong to the recorded position.

The eventual machine-output protocol therefore needs unambiguous assignment and removal. Rich structured state still has not earned scope.

### 4. Verification ergonomics are simpler than an internal state machine

We do not need persisted entered/unverified/verified sub-states.

At position 2, the UI can display:

~~~text
Current position: 2
Last verify: failed
[Verify] [Back] [Next]
~~~

or after a pass:

~~~text
Current position: 2
Last verify: passed
[Verify] [Back] [Next]
~~~

The last result explains what happened. Next still reruns verify before invoking step 3 up, so the displayed result is not a lock or token that Control Tower must keep authoritative.

### 5. Departure gating simplifies context publication

step 2 up can publish record_id and position 2 can become current immediately after successful up execution. step 2 verify then reads the ordinary current context.

There is no need to keep a hidden “candidate context” waiting for verification before the pointer advances.

### 6. Failure handling stays dumb

If verify fails:

~~~text
Recorded position: 2
Verify step 2: failed
Next: not run
~~~

The context remains available. The author decides whether to inspect, change code, verify again, or go down.

If up/down itself fails, the pointer does not move for that transition. The unresolved question is only what happens to machine output emitted by a process that later exits nonzero.

## What remains open

- Does “run to end” verify the final position as a completion check?
- Can verify publish context, or is its machine-output channel ignored/read-only by convention?
- What machine-output encoding handles set/remove with the least authoring friction?
- What happens to machine output emitted by a failed up/down process?
- Does the convention-only filesystem survive the first real project recipe?

These are narrower questions than the workflow semantics we started with.

## Next design decision

With verification timing settled, return to [Open questions and validation](open-questions.md). The next useful decision is the small context publication contract, especially failed-process output.
