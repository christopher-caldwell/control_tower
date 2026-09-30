---
id: CT-QUESTIONS
title: Open questions and validation
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- current-design.md
- discovery-brief.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
---

# Open questions and validation

The project is ready for its first formal discovery round. These questions should be investigated through the concrete UUID-file fixture rather than answered abstractly first.

## Q1 — Does the four-role step remain the right convention?

Leading candidate:

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

The owner considers this the leading mechanism, subject to change.

Test it rather than adding more roles.

## Q2 — What is the smallest UUID handoff?

The discovery fixture only needs one stage-1 UUID to be available to stages 2 and 3 and their verifiers.

Use the smallest implementation that works. Do not design a generalized context platform first.

## Q3 — Does filesystem-only authoring hold up?

Start with numbered directories and fixed executable-role filenames.

Add config only if the actual fixture exposes a concrete need.

## Q4 — Does the hexagonal boundary remain clean?

Validate that:

- CLI remains a driving adapter,
- transition behavior stays in application/core,
- state storage remains behind the core-owned port,
- process execution remains an outbound concern,
- SQLite remains an adapter detail and a memory test adapter can be swapped in without moving business rules.

## Removed from the first discovery

- auxiliary action abstraction,
- SQLite behavior beyond the minimal v0 adapter needed for CLI state,
- Tauri/HTTP,
- generalized source/candidate context contract,
- crash recovery,
- structural drift protection,
- concurrency/multi-instance behavior,
- workspace reset semantics.

See [First formal discovery brief](discovery-brief.md) for the complete seed.
