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
- ../research/discovery-01.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
---

# Open questions and validation

The first collaborative discovery is in progress. Findings and inspected source revisions are recorded in [the discovery record](../research/discovery-01.md). Resolve product choices one at a time; implementation mechanics remain delegated to discovery.

## B1 — Manual down after up itself fails

The known behavior covers a successful up followed by a failed verifier. One ordinary script-failure case remains unclear:

~~~text
02 completed; the UUID file contains hello
03/up appends to you, then exits nonzero
03/verify-up is not run
02 is still the last completed step
~~~

Should the user be able to explicitly run `03/down`, followed by optional `03/verify-down`, to back out that attempt?

The recommendation is yes, using the already-known UUID. That does not mean automatic cleanup, retry, success inference, or recovery after the Rust process crashes. The alternative is to leave cleanup of a nonzero up entirely outside the workbench for v0.

**Status:** awaiting owner decision. Do not publish the recommendation as accepted implementation authority. No automatic cleanup or new general recovery mechanism has been selected.

## Q1 — Does the four-role step remain the right convention?

Leading mechanism confirmed by the owner, subject to testing and change:

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

Test it rather than repeatedly asking for approval or adding more roles.

## Q2 — What is the smallest UUID handoff?

The discovery fixture only needs one stage-1 UUID to be available to stages 2 and 3 and their verifiers. Preserve it between normal CLI invocations through the selected SQLite storage adapter.

Use the smallest mechanism that works. Context checkpoints, patches, and source/candidate public interfaces are hypotheses, not prerequisites.

## Q3 — Does filesystem-only authoring hold up?

Start with numbered directories and fixed executable-role filenames. Add config only if the fixture exposes a concrete need. CLI names and flags remain adjustable at the entry layer.

## Q4 — Does the hexagonal boundary remain clean?

Use the pinned playbook sources in the discovery record. CLI delivery stays outside application behavior. Storage and process execution are outbound capabilities. SQLite remains an adapter detail. Apply active rules without importing absent authentication, hosted infrastructure, or production migration safeguards.

## Removed from the first discovery

- auxiliary action abstraction,
- SQLite behavior beyond the minimal v0 adapter needed for CLI state,
- Tauri/HTTP,
- generalized source/candidate context contract,
- crash recovery,
- structural drift protection,
- concurrency/multi-instance behavior,
- workspace reset semantics.

See [First formal discovery brief](discovery-brief.md) for the seed. Resolving these questions does not itself authorize Build or create an importable handoff.
