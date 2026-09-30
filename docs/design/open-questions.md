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

The first collaborative discovery is complete enough for an implementation handoff. The remaining items are implementation experiments or deferred edge cases, not blockers that must be resolved before the first working slice.

## Implementation experiment — does the four-role step remain the right convention?

Leading mechanism:

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

Build the UUID-file fixture with this convention and simplify only if real implementation evidence justifies it.

## Implementation experiment — smallest UUID handoff

The first fixture needs one stage-1 UUID to be available to stages 2 and 3 and their verifiers across normal CLI invocations.

Use the smallest mechanism that works. Do not design a generalized context platform first.

## Implementation experiment — filesystem-only authoring

Start with numbered stage directories and fixed executable-role filenames. Add config only if the implemented fixture exposes a concrete need.

## Implementation experiment — keep the hexagonal boundary clean

Validate that:

- CLI remains the only v0 driving adapter,
- transition behavior stays in Application,
- state persistence stays behind an Application-owned port,
- process execution stays behind an outer capability boundary,
- SQLite remains an adapter detail,
- the Entry executable owns explicit composition.

## Deferred edge cases

Do not block the first working version on richer recovery behavior.

In particular, if an up/down executable itself exits nonzero, v0 only needs to report the failure, stop automatic movement, and avoid running later stages automatically. Whether the user should receive special same-stage cleanup/recovery commands after a partially effective failed mutation is deliberately deferred until implementation experience makes the need concrete.

Also deferred:

- crash recovery/reconciliation,
- directory-structure drift protection,
- concurrency/multi-instance behavior,
- workspace reset semantics,
- Tauri/HTTP,
- helper ecosystem,
- generalized context/value semantics.

See [First formal discovery brief](discovery-brief.md) and [First discovery record](../research/discovery-01.md) for the complete scope and evidence.
