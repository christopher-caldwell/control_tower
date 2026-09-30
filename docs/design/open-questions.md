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

## Settled: target verification before step commit

For forward transition N-1 -> N:

~~~text
N/up
N/verify
commit N
~~~

The recorded step does not change until verify passes. A successful up followed by failed verify creates a pending transition rather than a committed step.

See [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).

## Q1 — What can the developer do while a transition is pending?

This is now the first design question.

Use pending step 002 after up succeeded and verify failed. Evaluate these candidate actions:

- Verify Again — rerun 002/verify without rerunning 002/up.
- Inspect — run auxiliary inspection using effective committed + pending context.
- Undo Pending — run 002/down using the effective context; discard pending changes only if it succeeds.
- Rerun Up — probably unavailable by default because create-style up actions may duplicate state.

**Exit criterion:** a failed verify does not force the user to recreate the fixture or manually rediscover IDs, and the UI remains obvious rather than workflow-engine-like.

## Q2 — What happens to machine output when up itself fails?

This is distinct from a failed verify.

If up emits an ID and later exits nonzero, should the value become pending context, remain recovery evidence only, or be ignored?

**Exit criterion:** one simple rule handles a partially successful external create without pretending the recorded step advanced.

## Q3 — What exactly does an action publish?

The concrete example requires scalar values such as user_id and record_id plus explicit removal.

Compare a tiny set/unset text protocol with a small JSON change document only after Q1 clarifies pending-context lifecycle.

**Exit criterion:** shell and Node can set an ID; down can remove it; malformed output has deterministic behavior.

## Q4 — What are verify's exact I/O rules?

verify clearly needs read access to effective pending context and exit status controls pass/fail.

Test whether verify ever needs to publish context. If no concrete case appears, keep it read-only at the Control Tower context boundary.

**Exit criterion:** a failed verify can be retried after a code change without any hidden side effects in workbench context.

## Q5 — How should backward transitions verify?

The forward rule does not automatically settle backward semantics.

For committed 003 -> 002 compare:

~~~text
003/down
commit 002
~~~

against:

~~~text
003/down
002/verify
commit 002
~~~

Do not choose symmetry for its own sake. Test whether previous-state verification helps or blocks recovery.

## Q6 — Does convention-only authoring remain sufficient?

Numeric directories plus up/down/verify still express the three-step lifecycle. Keep config out until a real metadata need appears.

## Validation order

Work through Q1 first. Pending-transition ergonomics determine the required context lifecycle. Then settle failed-up output and encoding. Backward verification can be tested after the forward path feels natural.

These are design steps, not implementation milestones.
