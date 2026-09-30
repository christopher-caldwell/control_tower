---
id: ADR-0003
title: Separate completed context, in-progress context, and process I/O
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-unapproved-details
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../history/2026-09-30-initial-design.md#e16-completed-and-in-progress-steps
- ../research/existing-tools.md
- ../design/three-step-workspace.md
---

# ADR-0003: Separate completed context, in-progress context, and process I/O

## Standing

The general context/output separation remains the leading candidate.

The step model now gives the storage problem clearer names:

- **completed context** — values associated with the last completed step,
- **in-progress changes** — values produced by the next step's successful up before verify succeeds,
- **effective context** — completed context plus in-progress changes.

The exact persistence format and process wire protocol remain undecided.

## Example

~~~text
completed: 02

completed context:
  user_id: 123
  record_id: 456

03/up succeeds and produces:
  mutation_id: 789

03/verify fails

in progress: 03
effective context:
  user_id: 123
  record_id: 456
  mutation_id: 789
~~~

03/verify, inspection actions, and 03/down need access to that effective context.

## Promotion and abandonment

If 03/verify later succeeds, promote 03's working changes into completed context and set completed = 03.

If 03/down succeeds while 03 is in progress, clear 03's working changes and keep the completed 02 context.

This is the simplest candidate behavior for a pending/in-progress step.

## The harder backward-context case

Moving down from a **completed** step is different because that step's outputs are already in completed context.

For example, if completed 02 introduced record_id, then 02/down returning to 01 must leave context appropriate for 01.

Two approaches remain worth testing:

- down explicitly publishes set/unset changes,
- Control Tower retains enough per-step context delta/snapshot information to restore the earlier completed context after successful down.

Do not select either until the three-step scripts make the ergonomics concrete. This is now a more meaningful question than JSON versus KEY=value.

## Process channels

The leading boundary remains:

~~~text
effective context -> executable
stdout/stderr -> human-visible result
machine output -> proposed context changes
exit status -> operation result
~~~

verify needs effective context and can use exit status as pass/fail. No real example yet requires verify to publish context changes.

## Failed up output

An up may emit an identifier and then exit nonzero.

That case remains distinct from a successful up followed by failed verify. The latter clearly creates an in-progress step. The former still needs a policy for whether emitted data becomes recovery evidence or usable working context.

## Persistence

An in-progress step may remain while the developer edits application code, so persistence across ordinary UI restarts is increasingly useful.

The backend remains open. The data model should stay small: completed step, optional in-progress step, completed context, in-progress changes, and recent execution results.

## Next validation

Exercise both:

1. verify failure -> retry verify -> complete,
2. verify failure -> down -> return to previous completed step.

Then test completed-step down to determine whether author-published context changes or stored per-step deltas produce the simpler contract.
