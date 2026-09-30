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

This is a decision queue, not a release plan. Resolve one coherent question at a time. The three-step probe has deliberately reduced several earlier questions instead of adding machinery.

## Q1 — What exactly does an action publish?

The concrete example currently requires only scalar values such as user_id and record_id plus an explicit way to remove them.

Do not require nested objects, multiline strings, null semantics, a type system, or many scopes until a real authoring example needs them.

Compare the smallest useful KEY=value/set-unset convention with a small JSON change document. stdout/stderr must remain ordinary process output.

**Exit criterion:** a shell step and a Node step can set an ID; a down step can remove it; malformed output has deterministic behavior.

See [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## Q2 — What happens to machine output when the process fails?

The migration pointer rule is settled: a failed transition does not move the recorded position. What remains is narrower: if the executable emitted machine output before returning nonzero, should that output be merged, retained only with the failed result, or ignored?

Use one example where an external create succeeds, emits an ID, and then the script exits nonzero.

**Exit criterion:** the behavior is simple enough to explain in one sentence and does not pretend to recover external state.

## Q3 — When does an assertion gate run?

This is the next design decision because it controls both pointer meaning and context timing.

The three-step probe identifies two plausible rules:

1. **Arrival check:** up -> verify target -> move pointer.
2. **Departure gate:** up -> move pointer; verify current position before the next up.

The original “position 2 must meet x,y before position 3” example points naturally toward a departure gate. An arrival check makes a recorded position carry stronger meaning. Both are small; they fail differently.

**Exit criterion:** choose one rule, define final-step behavior, and walk a failed assertion once in each direction.

See [the concrete comparison](three-step-workspace.md#4-assertions-expose-the-first-real-semantic-choice).

## Q4 — How are context inputs exposed?

Decide whether executables receive only a context-file path, automatic scalar environment variables, command-line bindings, or a small combination.

The same authored script should remain understandable and runnable outside the workbench with explicit inputs. Avoid a variable-precedence system unless a real collision requires it.

**Exit criterion:** step 2 can consume user_id from step 1 in shell and Node without an SDK.

## Q5 — What local persistence is necessary?

The recorded pointer and context probably need to survive UI restarts to make the workbench pleasant, but the backend is not selected.

Test closing/reopening with position 2 and two IDs. Also test what happens after the authored migration directories change order or names. A saved integer is not enough if the migration set itself changed.

**Exit criterion:** reopen without pretending an old pointer belongs to a changed migration set. Do not build event sourcing.

## Q6 — Does a convention-only workspace remain sufficient?

The three-step example needs no YAML/TOML file: numeric directories give order; up/down names give direction; shebangs give runtimes.

Try the convention before introducing config. Write down the first requirement that genuinely cannot be expressed well by the filesystem. Likely candidates include display metadata, workspace-level environment selection, or working-directory overrides, but none is yet accepted.

**Exit criterion:** either keep convention-only authoring for the first prototype or identify a concrete metadata field that justifies a workspace config file.

## What is no longer an open product question

The Dagu comparison has served its main purpose. The owner tried it and found it close in capability but intentionally heavier than the desired workbench. Control Tower does not need to prove market uniqueness or become a smaller clone.

Likewise, the core does not need a generic rollback guarantee, a login system, hosting, a scheduler, workers, or built-in HTTP/Postgres/Node execution types.

## Validation order

Resolve Q3 first because assertion timing affects when context becomes current. Then settle Q1 and Q2 together with the three-step scripts. Q4 and Q6 should fall out of that authoring exercise. Choose a persistence backend only after the runtime contract is small and stable.

These are research/design steps, not committed implementation phases or dated milestones.
