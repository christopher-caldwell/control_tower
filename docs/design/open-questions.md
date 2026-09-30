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
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
---

# Open questions and validation

This is a decision queue, not a delivery schedule, task breakdown, or request for the owner to answer everything at once. Resolve one coherent question at a time, and update the linked ADR rather than opening a new design document for every reply.

## Q1 — What exactly does an action publish?

**First discussion.** Decide the output envelope and its semantics together: assigning values, removing keys, representing null, handling repeated keys, and recognizing incomplete output. Choose between `KEY=value`, a JSON document, or JSON Lines based on actual authoring, not compact examples alone.

Use one shell executable and one Node executable to carry a fixture ID, a nested record, a multiline string, and a value that must later be removed. Compare a whole replacement object against explicit updates. Check whether an SDK, wrapper command, or expression DSL becomes necessary. None has been accepted.

**Exit criterion:** one unambiguous protocol handles those cases, keeps stdout/stderr available for ordinary output, and documents what counts as a complete publication. See [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## Q2 — What happens after a partial mutation?

This question must be settled alongside Q1 before calling the protocol complete. A creation action can succeed at the API, write a useful ID, then fail. Alternatively, it can exit successfully and fail its stage assertion. A crash can occur before the ID is published at all.

Compare keeping outputs as recovery evidence with publishing them into the active context. A verifier may need proposed IDs before a stage can be accepted. Neither discarding evidence nor automatically trusting it is an adequate blanket rule.

**Exit criterion:** walk through nonzero exit, invalid output, verification failure, and interruption. The design must retain available recovery identifiers, avoid a false verified-stage label, and avoid silently rerunning a possibly completed mutation. No mechanism can recover an identifier that was never emitted without author-supplied external inspection.

## Q3 — What does navigation promise?

Confirm whether checks belong to stages, transitions, or both; whether every stage needs a check; and whether reaching a target verifies each intermediate stage. Clarify the distinction between continuing from the current fixture and starting a fresh run.

Use a reversible transition, a deliberately irreversible action, and a down action that fails. Include an inspection and an out-of-band mutation. Ask whether the migration-like model remains useful without turning every utility into a formal transition.

**Exit criterion:** source/target conventions, unavailable-down behavior, uncertain-state handling, and rebuild behavior can be explained on one linear example. See [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).

## Q4 — How are inputs scoped and exposed?

Separate environment configuration, invocation-specific inputs, session fixture values, and controller metadata. Decide whether scalar environment projections are needed at all, then define mapping and reserved names if they are. Confirm the working directory and runtime environment for UI launches.

**Exit criterion:** the same script can be run from the workbench or manually with explicit inputs. Switching target environments cannot silently reuse fixture IDs without an explicit policy. Sensitive configuration does not automatically enter saved snapshots or logs. Session-to-workspace/environment binding remains an open design choice, not an inferred feature.

## Q5 — What persistence is necessary?

Decide whether restart persistence is essential for the first experiment and where session data lives. The backend is not selected: JSON files and SQLite are both possible. A JSON process protocol does not choose a JSON database.

Include closing and reopening the UI, an interrupted write, and a changed workflow definition. Define how a stored stage is associated with the recipe that gave it meaning. A saved stage number alone is insufficient after reordering stages or changing their checks.

**Exit criterion:** define enough retained context and attempt evidence to reopen honestly, without promising durable external execution or creating a general event-sourcing system.

## Q6 — Does the interaction justify a custom tool?

Try one generic fixture recipe in Dagu using ordinary executables, not its database/HTTP actions. Compare create, inspect, change application code, rerun one action, compensate, and rebuild. Record the installed version and actual UI steps; current documentation alone cannot establish the click experience.

**Exit criterion:** identify specific friction the proposed Control Tower interface removes, or acknowledge that Dagu is sufficient. Do not require a market-wide uniqueness claim for a personal project. Native versus web UI should follow this exercise, not precede it.

## Proposed validation sequence

First settle Q1/Q2 with small authoring examples and explicit failure traces. Then use one fixture recipe to settle Q3/Q4. Decide the minimum persistence behavior from that experience. The Dagu trial can proceed independently throughout.

These are research steps, not committed implementation phases, acceptance criteria for a release, or dated milestones. No implementation deadline has been agreed.
