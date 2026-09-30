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
---

# Three-step workspace design probe

## Purpose

This is a concrete design exercise, not accepted syntax. Its job is to make abstractions earn their place.

The example models a common development loop:

1. Create a test user.
2. Create an associated record in the state required by the ticket.
3. Invoke the mutation under development.

The developer can inspect the fixture at any point, change application code outside Control Tower, move down to an earlier position, and run forward again.

## Candidate workspace shape

The smallest useful filesystem may need no central configuration file at all:

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

The names up, down, verify, the numeric ordering, and the actions directory are illustrative. They are intentionally boring because the shebang inside each executable chooses how the operation runs.

A convention-first layout is worth testing before introducing YAML/TOML. A config file may later earn its place for display labels, environment selection, working-directory overrides, parameters, hidden/disabled steps, or other metadata that cannot be expressed cleanly by convention.

## Position and context through the example

The recorded position and authored context are separate:

| Recorded position | Meaning in this example | Context that may be useful |
| --- | --- | --- |
| 0 | Nothing applied by Control Tower | empty |
| 1 | create-user up succeeded | user_id |
| 2 | create-associated-record up succeeded | user_id, record_id |
| 3 | mutate-record up succeeded | user_id, record_id |

The table describes Control Tower bookkeeping, not a guarantee about an external API or database.

### Step 1 — create user

up creates the fixture user and publishes user_id.

down removes that user and should remove user_id from the carried context.

verify, if this convention survives, checks whatever the author considers necessary about the user.

### Step 2 — create associated record

up uses user_id, creates the required associated data, and publishes record_id.

down removes or otherwise compensates for that record and should remove record_id from context.

verify can check the preconditions that make the fixture useful for the next operation.

### Step 3 — mutate record

up uses record_id to invoke the operation under development.

down is the author's chosen compensating operation: perhaps reopen the same record, reset fields, or call a project-specific repair script. Control Tower does not know whether it is a true inverse.

This step may not need to publish any new context.

### Auxiliary inspect action

actions/inspect reads the current context, queries whatever systems the author wants, and prints useful state to stdout. Running it does not change Control Tower's recorded position.

Control Tower does not enforce that inspect is actually read-only. The no-position-change rule is bookkeeping, not a sandbox or semantic guarantee.

## Walking the migration set

From position 0 to position 3:

~~~text
run step 1 up
  success -> record position 1

run step 2 up
  success -> record position 2

run step 3 up
  success -> record position 3
~~~

From position 3 back to position 1:

~~~text
run step 3 down
  success -> record position 2

run step 2 down
  success -> record position 1
~~~

Any nonzero exit stops the walk. The position does not move for the failed transition. The external system may nevertheless have been partly changed; that is the same category of author responsibility as a failed migration.

A missing down file means that path cannot be traversed backward through the normal migration mechanism. Control Tower should not silently decrement the position.

## What this example teaches us

### 1. A separate “stage” abstraction is probably unnecessary

The migration index already gives us the useful model. Step N is the move between positions N-1 and N. Introducing both stages and steps adds vocabulary without adding capability in this example.

### 2. Central configuration has not earned its way in yet

Order comes from the numeric prefix. Direction comes from up/down file names. Runtime comes from the shebang. The workspace directory itself is already a configuration surface.

This is not a decision to ban configuration. It means the first config field should solve a concrete problem rather than exist because workflow products usually have one.

### 3. Context needs both set and remove

Forward execution needs to carry user_id and record_id. Backward execution needs to discard values that no longer identify the current fixture.

That means the eventual output protocol needs an unambiguous removal operation in addition to assignment. This requirement is stronger than the current evidence for nested JSON, multiline values, or typed variables.

### 4. Assertions expose the first real semantic choice

The user originally described assertions as gates:

~~~text
0 -> 1 -> 2 [must meet x,y] -> 3
~~~

Two small interpretations survive the example.

**Arrival check:** run step 2 up, run step 2 verify, and only then record position 2.

- Advantage: a recorded position can mean its configured check passed.
- Cost: verify may need user_id/record_id produced by the just-finished up before the position is committed. If verify fails, the external mutation and generated IDs may already exist while the recorded position remains 1.

**Departure gate:** step 2 up succeeds and records position 2. Before allowing step 3 up, run step 2 verify.

- Advantage: the migration pointer remains very simple and the check matches the original “must meet x,y before 3” wording.
- Cost: being at position 2 does not imply verification has passed. The final position also needs an explicit Verify action or an end-of-run check if postconditions matter.

Neither has been accepted. This choice should be made before finalizing how action outputs are committed.

### 5. We can keep failure handling dumb

The example does not require Control Tower to model “unknown external state” as a separate state-machine branch.

A sufficient failure display can be:

~~~text
Recorded position: 1
Last attempt: 1 -> 2
Result: failed (exit 1)
~~~

The author decides whether to inspect, repair, retry, run a compensating action manually, or start over.

### 6. The first context protocol can be smaller than we were assuming

This example needs two scalar identifiers plus deletion. It does not yet justify arbitrary nested data, an expression language, a type system, many scopes, or event-sourced state.

A context file plus a separate output file remains a good candidate, but the output encoding should be chosen against this small need first.

## What is intentionally absent

There is no login, scheduler, server/worker distinction, DAG, retry policy, database adapter, HTTP adapter, Node runtime adapter, or workflow expression engine in this probe.

Future helpers can make common authoring jobs pleasant, but they should remain executables that consume the same core contract.

## Next design decision

Resolve the assertion timing choice above. It directly determines when generated context becomes current, what a recorded position means, and how a full run behaves at the final step.

After that, return to [Open questions and validation](open-questions.md) for the smallest set/unset output protocol.
