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
- ../history/2026-09-30-initial-design.md#e18-four-file-gauntlet
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

## Runtime model

Completed steps form a stack of context checkpoints:

~~~text
0  {}
1  { user_id }
2  { user_id, record_id }
~~~

At most one directional transition can be active.

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
push completed checkpoint 03
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
pop 03 and settle checkpoint 02
~~~

## How candidate context is built

### Forward candidate

Start from the source completed checkpoint and apply the up mutation's context patch.

~~~text
candidate_N = checkpoint_(N-1) + up_patch_N
~~~

### Backward candidate

Start from the already-saved target checkpoint and apply any down mutation patch.

~~~text
candidate_(N-1) = saved_checkpoint_(N-1) + down_patch_N
~~~

The down patch is optional.

This is the key compromise that survived the gauntlet: automatic restoration for the normal case, but an escape hatch when down reconstructs an equivalent lower state with new identifiers.

## Why both source and candidate are needed

Consider:

~~~text
02 checkpoint:
  user_id: 123
  record_id: 456
~~~

02/down deletes record 456.

The candidate 01 context should not contain record_id. But 02/verify-down may need 456 to query:

~~~text
does record 456 no longer exist?
~~~

So verify-down needs the source context even though it validates the candidate lower state.

The same issue can happen in the up direction when an up replaces an existing identifier.

The exact process API is open, but both logical views are justified:

~~~text
source context
candidate context
~~~

## Gauntlet

### 1. Happy path forward

~~~text
01/up
01/verify-up
commit 01

02/up
02/verify-up
commit 02
~~~

**Result:** clean. Push a context checkpoint after each verified transition.

### 2. verify-up fails

~~~text
02 completed
03/up succeeds
03/verify-up fails
~~~

Keep completed 02. Keep active up transition 03 with its candidate context.

Allowed recovery:

~~~text
Inspect
Verify Up Again
Down
~~~

Do not rerun 03/up automatically.

**Result:** clean.

### 3. Back out an in-progress up

~~~text
02 completed
03/up succeeds
03/verify-up fails
03/down succeeds
03/verify-down succeeds
~~~

Discard the 03 candidate and return to completed 02. If down emitted a patch, apply it to the saved 02 checkpoint before settling.

**Result:** clean.

### 4. Normal completed down

~~~text
03 completed
03/down
03/verify-down
=> 02 completed
~~~

Restore the 02 checkpoint, patched by any outputs from 03/down.

**Result:** clean.

### 5. verify-down fails

~~~text
03 completed
03/down succeeds
03/verify-down fails
~~~

Keep completed pointer at 03. Keep an active down transition with source 03 and candidate 02 contexts.

Allow Inspect and Verify Down Again.

Do not rerun 03/down automatically.

**Result:** clean, but recovery beyond verifier retry remains intentionally manual.

### 6. Down creates replacement target data

Old 02 checkpoint:

~~~text
record_id: 456
~~~

03/down cannot restore 456 and instead creates 789.

Down output patch:

~~~text
record_id: 789
~~~

Candidate 02 checkpoint becomes the historical 02 checkpoint overlaid with 789.

**Result:** pure snapshot restore would fail; checkpoint + patch survives.

### 7. Higher step overwrites an existing key

~~~text
01: user_id = A
02/up changes user_id = B
~~~

The 02 checkpoint records B. A verified 02/down can restore 01 checkpoint A unless down emits an override.

**Result:** checkpoint stack handles shadowing naturally.

### 8. Mutation exits nonzero after partial external work

Example: create succeeds remotely, script prints an ID, then exits 1.

Do not advance the pointer or automatically create a normal active transition. Preserve the execution result and any captured machine output as recovery evidence.

A later “adopt/verify anyway” recovery feature may be useful, but it is not required to keep the core model sound.

**Result:** unavoidable external-side-effect edge; not a design failure.

### 9. Control Tower crashes during mutation

Persist transition intent before process launch. On restart, mark the attempt interrupted/unknown and never auto-retry.

The developer can inspect external state and choose recovery.

**Result:** requires durable local bookkeeping, not orchestration machinery.

### 10. Control Tower crashes after mutation succeeds but before verifier completes

The active transition and candidate context must already be persisted. Restart can resume with Verify Up/Down Again.

**Result:** checkpoint/active-transition model handles it.

### 11. Step scripts change while debugging

Changing verify-up after it failed is expected. Hard checksums on executable contents would fight the intended workflow.

Record execution details if useful, but do not block merely because file contents changed.

**Result:** intentionally differs from production migration tooling.

### 12. Step structure changes

Adding/removing/reordering numbered step directories can invalidate saved checkpoint meaning.

Persist the step sequence identity and detect structural drift.

**Result:** needs a guardrail, but no central config is required.

### 13. Two app instances operate the same workspace

Without a lock, both could mutate the pointer/checkpoint stack.

Use one local writer lock.

**Result:** small implementation requirement.

### 14. Verifier absent

Mutation exit 0 completes the directional transition immediately.

**Result:** optional verification remains coherent.

### 15. Down absent

Backward navigation through that step is unavailable.

**Result:** coherent; no fake rollback.

## Verdict

The four-role step survives the gauntlet.

The **pure snapshot** context model does not. The stronger version is:

> completed context checkpoints + one active directional transition + optional mutation patch.

This stays small while handling repeated navigation, directional verification, replacement IDs, failed verification, and context rewind.

## Remaining pressure tests

- exact patch/output encoding,
- auxiliary actions publishing context,
- explicit recovery after interrupted/nonzero mutation,
- storage backend and retention,
- how structural drift is surfaced in the UI.
