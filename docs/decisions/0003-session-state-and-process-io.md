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

The exact protocol is still unaccepted. File names such as WB_CONTEXT or WB_OUTPUT, JSON shapes, persistence backend, environment projection, and failure-publication rules remain working ideas.

## What the three-step probe actually requires

The first concrete workspace only needs:

~~~text
after step 1:
  user_id

after step 2:
  user_id
  record_id

after step 3:
  user_id
  record_id
~~~

Walking down step 2 must be able to remove record_id. Walking down step 1 must be able to remove user_id.

This is important evidence: assignment and removal are required. Nested objects, multiline values, arbitrary typing, variable precedence, and an expression language are not yet required.

## Distinct responsibilities

| Information | Owner | Current direction |
| --- | --- | --- |
| Recorded migration position | Control Tower | Internal bookkeeping; scripts do not set it directly. |
| Shared authored context | Control Tower plus explicit script outputs | Small values intentionally carried between actions. |
| Process configuration | Author/environment | Do not silently copy the entire inherited environment into durable context. |
| stdout/stderr | Executable | Human-visible execution result. |
| Machine output | Executable through a defined channel | Candidate context set/remove operations. |
| Large files/artifacts | Author/filesystem | No managed artifact system has earned scope. |

The table is conceptual. It does not imply separate databases or a large state subsystem.

## Input candidates

Environment variables are excellent for simple scalar transport and for pointing to a context file, but they are not a parent-process storage mechanism. Command-line arguments work well for existing executables but make generic binding verbose. stdin is structured but consumes a useful stream. A read-only context snapshot file remains a strong language-neutral candidate.

The three-step example does not yet prove that every scalar context value should be automatically projected into the environment. That convenience can be tested later.

## Output candidates

The strongest decision so far is to keep machine output separate from stdout/stderr.

The smallest real requirement is now:

- set a scalar key,
- remove a scalar key,
- detect malformed/incomplete output,
- keep normal stdout/stderr untouched.

A KEY=value-style file is attractive for shell authoring but needs a deletion convention. A JSON document can express set/remove clearly but introduces a parser and structure that may be more than v0 needs. JSON Lines and SDK/RPC mechanisms remain unjustified by the example.

Do not choose a format merely because another workflow product uses it.

## Assertion timing affects context timing

The three-step probe surfaced a dependency between ADR-0002 and this proposal.

If assertions are **arrival checks**, step 1 up may create user_id and step 1 verify must see it before Control Tower records position 1.

If assertions are **departure gates**, step 1 up can publish user_id and record position 1 immediately; verification happens only before walking onward.

That choice should be resolved before specifying exactly when machine output is merged into active context.

## Failed process output remains the hard edge

A process can change an external system, write an identifier, and then exit nonzero. The migration-runner philosophy says Control Tower does not need to infer external truth, but it still must choose what to do with any machine output it received.

The minimum honest options are:

- keep failed output visible as part of the result but do not merge it,
- merge some/all failed output into context,
- or require the author to use a separate recovery mechanism.

No choice is accepted yet. This is a much smaller question than building a generalized recovery state machine.

## Persistence

Restart persistence for the pointer/context is still a candidate, not a decided storage implementation. JSON files, SQLite, or another local mechanism can be evaluated later.

The process protocol must not decide the persistence backend by accident. Likewise, local execution does not make the context a secret store.

## Confirmation

After assertion timing is settled, test the protocol against the three-step workspace using one shell step and one Node step. Require only scalar set/remove behavior first. Add structured values only when a concrete authoring case cannot be expressed cleanly without them.
