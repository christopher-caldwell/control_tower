---
id: ADR-0003
title: Separate committed context, pending transition context, and process I/O
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-unapproved-details
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../history/2026-09-30-initial-design.md#e15-verify-before-step-commit
- ../research/existing-tools.md
- ../design/three-step-workspace.md
---

# ADR-0003: Separate committed context, pending transition context, and process I/O

## Standing

The owner endorsed the general context/output separation as the leading candidate. Verification-before-step-commit adds one concrete requirement: successful up outputs may need to exist **before** they are committed to the settled workspace context.

The exact wire format and storage backend remain unaccepted.

## Required context views

The three-step example now implies two logical context layers.

**Committed context** belongs to the last committed step.

**Pending changes** come from a successful target up whose verify has not yet passed.

The effective context seen by the pending target is conceptually:

~~~text
committed context + pending changes
~~~

For example:

~~~text
committed step: 001
committed:
  user_id: 123

pending step: 002
pending:
  record_id: 456

effective for 002/verify:
  user_id: 123
  record_id: 456
~~~

This is a logical distinction. It does not require two database tables or a sophisticated transaction engine.

## Commit behavior

On target up success, parse and retain its machine-output changes as pending.

Run target verify against the effective context.

If verify succeeds, promote the pending changes into committed context and change the recorded step.

If verify fails, retain pending changes so verify, inspect, and a pending undo can still use them.

If a pending undo succeeds, discard the pending changes and keep the previous committed context.

This lifecycle is now more important than choosing JSON versus KEY=value.

## Process channels

The leading process boundary remains:

~~~text
context input -> executable
stdout/stderr -> human-visible result
machine output -> proposed context changes
exit status -> operation result
~~~

A context snapshot file remains a strong language-neutral candidate. Environment variables may locate it or expose convenience scalars.

The machine-output channel still needs assignment and removal semantics. The three-step example does not justify richer data types yet.

## Verify process contract

verify must receive the effective pending context so it can inspect values created by up.

The smallest useful verify contract is otherwise read-only from Control Tower's perspective: stdout/stderr for explanation and exit status for pass/fail.

Whether verify is allowed to publish additional context remains open and currently has no motivating example.

## Failed up output

A separate edge remains: an up can emit machine output and then exit nonzero.

That is different from “up succeeded, verify failed.” In the latter case pending context clearly has a role. In the former case we still need to decide whether emitted values are retained as recovery evidence, promoted into pending context, or ignored.

Do not conflate the two cases.

## Persistence

If a pending transition can survive long enough for the developer to change code and retry verify, it likely needs to survive ordinary UI refresh/restart as well. That is now stronger evidence for local persistence than before, but the backend remains undecided.

A simple local file or SQLite can both represent committed step, pending step, pending changes, and recent results. Choose later based on implementation ergonomics, not protocol aesthetics.

## Confirmation

Use the pending-002 scenario:

1. committed 001 with user_id,
2. 002/up succeeds and produces record_id,
3. 002/verify fails,
4. inspect sees user_id + record_id,
5. restart the UI/process,
6. verify again or run 002/down,
7. either commit 002 or return cleanly to 001.

The smallest mechanism that supports that flow should become the initial context protocol.
