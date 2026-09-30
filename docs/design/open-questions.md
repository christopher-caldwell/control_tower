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
- three-step-workspace.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
---

# Open questions and validation

This is a decision queue, not a release plan.

## Under evaluation: four executable roles per step

The current proposal is:

~~~text
up
down
verify-up     # optional
verify-down   # optional
~~~

A directional mutation would change the completed pointer only after its optional directional verifier succeeds.

This looks cleaner than reusing a single verifier for both directions, but it remains a design candidate until the backward/context exercise confirms it.

## Q1 — Snapshot restoration or explicit down context output?

The four-file model makes this the first question.

For completed 02 -> 01, compare:

**Snapshot restoration**

~~~text
02/down
02/verify-down
restore saved completed-01 context
commit 01
~~~

against:

**Explicit down output**

~~~text
02/down publishes context set/unset changes
02/verify-down checks effective target
commit 01 with those changes
~~~

**Exit criterion:** record_id can remain available during verify-down but cannot remain stale after 01 becomes completed. Authoring should stay simple.

## Q2 — What exactly can happen during a failed verify-down?

If 02/down succeeds but 02/verify-down fails, completed remains 02 and a down transition remains active.

Confirm the minimum useful controls: Inspect and Verify Down Again. Determine whether an explicit Up-to-restore action needs first-class UI support or can remain manual/recovery behavior.

## Q3 — What happens to output when a mutation itself fails?

If up or down emits values and later exits nonzero, decide whether they become working recovery context or attempt evidence only.

## Q4 — Are both verifiers read-only?

No current example requires verify-up or verify-down to publish context.

Prefer read-only unless a real use case appears.

## Q5 — What exact machine-output encoding is smallest?

Defer JSON versus text set/unset until the snapshot-vs-explicit context decision is made. Snapshot restoration may dramatically reduce how much output syntax down needs.

## Q6 — Does convention-only authoring remain sufficient?

Four fixed filenames still fit the filesystem-first model. Do not add central config merely to list them.

## Validation order

Compare the two context strategies first using step 02. Then test verify-down failure ergonomics. Only after that choose the machine-output encoding.

These are design steps, not implementation milestones.
