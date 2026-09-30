---
id: ADR-0003
title: Separate persisted context from process input and output
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-unapproved-details
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../research/existing-tools.md
- ../design/three-step-workspace.md
---

# ADR-0003: Separate persisted context from process input and output

## Standing

The owner endorsed the general separation as the leading candidate for exploration: Control Tower owns small shared context, an executable receives an input view, stdout/stderr remain ordinary visible output, and a separate machine-output channel can update context.

The exact protocol is still unaccepted. File names, encoding, persistence backend, environment projection, and failed-process output rules remain working ideas.

## What the three-step probe requires

The first concrete workspace only needs:

~~~text
after step 1 up:
  user_id

after step 2 up:
  user_id
  record_id

after step 3 up:
  user_id
  record_id
~~~

Walking down step 2 needs to remove record_id. Walking down step 1 needs to remove user_id.

Assignment and removal are therefore required. Rich nested state, variable precedence, an expression language, and a large type system are not.

## Distinct responsibilities

| Information | Owner | Current direction |
| --- | --- | --- |
| Recorded migration position | Control Tower | Internal bookkeeping; scripts do not set it directly. |
| Shared authored context | Control Tower plus explicit script outputs | Small values carried between actions. |
| Process configuration | Author/environment | Do not copy the entire inherited environment into durable context. |
| stdout/stderr | Executable | Human-visible execution result. |
| Machine output | Executable through a defined channel | Candidate context set/remove operations. |
| Verify result | Verify executable exit status | Gate forward movement; latest result can be displayed. |

## Successful up/down and context timing

The verification decision removes one earlier complication.

When up N exits successfully, Control Tower records position N and applies the successful action's context updates according to the eventual output protocol. verify N runs later, immediately before any forward move to N+1.

Therefore verify N sees the normal current context. There is no need for a hidden candidate-context layer waiting for verification.

Similarly, successful down N can apply its context removals and then record position N-1.

Exact ordering between durable pointer write and durable context write is a persistence/atomicity implementation question, not a reason to add workflow semantics.

## Input candidates

Environment variables are convenient scalar transport and can point to a context file, but they are not durable parent-process storage. Command-line arguments fit existing executables but make generic binding verbose. stdin consumes a useful stream. A read-only context snapshot file remains a strong language-neutral candidate.

Automatic projection of every scalar into environment variables still has not earned acceptance.

## Output candidates

The strongest direction remains a separate machine-output channel from stdout/stderr.

The first real requirement is:

- set a scalar key,
- remove a scalar key,
- detect malformed output,
- keep normal stdout/stderr untouched.

KEY=value is shell-friendly but needs deletion syntax. A small JSON change document expresses set/remove clearly but may be more ceremony. Choose against real scripts, not feature breadth.

## Failed process output remains the hard edge

A process can change an external system, emit an identifier, and then exit nonzero. The migration-runner philosophy says the pointer does not advance, but it still leaves a narrow machine-output decision.

Options remain:

- retain failed output only with the result,
- merge it into current context,
- or ignore it and require external recovery.

No choice is accepted yet.

## Verify output is a separate question

The smallest verify contract is read current context, print results, and signal pass/fail through exit status.

Allowing verify to publish context would make a gate also mutate Control Tower state. That may be useful in some edge case, but the three-step example does not require it. Treat verify context publication as unearned until a concrete case demonstrates it.

## Persistence

Restart persistence for pointer/context remains a candidate. JSON files, SQLite, or another local mechanism can be evaluated later.

The process protocol must not accidentally choose the storage backend, and local execution does not make context a secrets store.

## Confirmation

Test the protocol against the three-step workspace using shell and Node. Require scalar set/remove first. Include a successful up followed by failed verify, and confirm the pointer/context remain usable for inspect -> code change -> verify again.

Then test a failed up that emitted an ID before exiting nonzero. That is now the main unresolved context edge.
