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

This is a decision queue, not a release plan. Resolve one coherent question at a time.

## Settled: forward verification gate

If the current step has verify, it must succeed immediately before Control Tower advances forward to the next step.

Successful up records the new position first. This lets the current step's verify read the context created by that up. A failed verify leaves the developer at the same position and blocks the next up.

See [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).

## Q1 — What exactly does an action publish?

The concrete example requires scalar values such as user_id and record_id plus an explicit way to remove them.

Do not require nested objects, multiline strings, null semantics, a type system, or many scopes until a real authoring example needs them.

Compare the smallest useful KEY=value/set-unset convention with a small JSON change document. stdout/stderr must remain ordinary process output.

**Exit criterion:** a shell step and a Node step can set an ID; a down step can remove it; malformed output has deterministic behavior.

See [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## Q2 — What happens to machine output when the process fails?

The pointer rule is settled: a failed up/down transition does not move the recorded position. What remains is narrower: if the executable emitted machine output before returning nonzero, should that output be merged, retained only with the failed result, or ignored?

Use one example where an external create succeeds, emits an ID, and then the script exits nonzero.

**Exit criterion:** the behavior is simple enough to explain in one sentence and does not pretend to recover external state.

## Q3 — What are verify's exact I/O rules?

Verification timing is settled, but its process contract is not.

The smallest model is that verify reads current context, emits stdout/stderr, and uses exit status as the pass/fail signal. It may be cleaner to ignore/disallow verify machine-output updates so a check cannot quietly mutate Control Tower context.

Test whether any real example needs verify to discover/publish a value. If not, keep it read-only at the Control Tower context boundary.

**Exit criterion:** define whether verify can publish context and define final-position manual/completion verification behavior.

## Q4 — How are context inputs exposed?

Decide whether executables receive only a context-file path, automatic scalar environment variables, command-line bindings, or a small combination.

The same authored script should remain understandable and runnable outside the workbench with explicit inputs. Avoid a variable-precedence system unless a real collision requires it.

**Exit criterion:** step 2 can consume user_id from step 1 in shell and Node without an SDK.

## Q5 — What local persistence is necessary?

The recorded pointer and context probably need to survive UI restarts, but the backend is not selected.

Test closing/reopening with position 2 and two IDs. Also test what happens after authored migration directories change order or names.

**Exit criterion:** reopen without pretending an old pointer belongs to a changed migration set. Do not build event sourcing.

## Q6 — Does a convention-only workspace remain sufficient?

The three-step example needs no YAML/TOML file: numeric directories give order; up/down/verify names give behavior; shebangs give runtimes.

Try the convention before introducing config. Write down the first requirement that cannot be expressed well by the filesystem.

**Exit criterion:** either keep convention-only authoring for the first prototype or identify a concrete metadata field that justifies workspace config.

## Validation order

Settle Q1/Q2 next because departure-gate semantics now make context publication straightforward on successful up/down. Q3 can then stay intentionally tiny. Q4 and Q6 should fall out of authoring the three-step scripts. Choose a persistence backend only after the process contract is stable.

These are design steps, not committed implementation phases or dated milestones.
