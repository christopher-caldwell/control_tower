---
id: ADR-0007
title: Use a desktop-only three-column workbench shell
type: decision
status: accepted
created: '2026-10-02'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
decision_authority: explicit-user-direction-after-ui-reference-review
decision_date: '2026-10-02'
sources:
- 0002-stage-navigation-and-verification.md
- 0006-loopback-web-ui.md
- ../research/2026-10-02-ui-reference-review.md
---

# ADR-0007: Use a desktop-only three-column workbench shell

## Decision

The first graphical Control Tower interface is a **desktop-only developer workbench**.

Its primary workflow screen uses three simultaneously visible regions:

```text
┌────────────────┬──────────────────────────────┬─────────────────────────┐
│ WORKFLOWS      │ STAGES                       │ STAGE INSPECTOR         │
│                │                              │                         │
│ left rail      │ primary execution context    │ persistent right rail   │
│                │                              │                         │
└────────────────┴──────────────────────────────┴─────────────────────────┘
                 sticky declarative action region
```

The shell deliberately preserves context. It is not a mobile-first or responsive content site.

## Terminology and launch scope

Use these terms consistently:

- **Workspace**: the enclosing developer environment. In the examples gallery, `simple`, `multi_language` and `py_capsule` are Workspaces.
- **Workflow**: a runnable Control Tower scratch/work area inside a Workspace, with its own stages and checkpoint state.
- **Stage**: one ordered migration-style transition within a Workflow.

> **Terminology update (2026-10-04, [issue #5](https://github.com/christopher-caldwell/control_tower/issues/5)):** this ADR originally named the hierarchy Project → Workspace → Stage. On explicit owner direction it is now **Workspace → Workflow → Stage**: the former Project is the Workspace and the former Workspace is the Workflow. This is a vocabulary change only; none of the decisions below changed. Dated records under `docs/research/` and `docs/history/` keep the earlier wording.

For UI v0, the directory from which the UI is launched establishes the **Workspace**. Control Tower discovers Workflows from that Workspace context and the user may switch among those Workflows in the UI.

Workspace switching is deferred. Do not introduce a global workspace registry, recent-workspace list, favorites, moved-path recovery or an “Open workspace” product model for v0. If repeated real use makes relaunching from another Workspace materially painful, Workspace switching can be reconsidered.

The exact filesystem algorithm for identifying Workflow candidates is an implementation detail to resolve against existing workspace/workflow conventions; this ADR does not authorize a new workflow manifest or global registry.

## Left workflow rail

The left rail is the workspace-local workflow navigator.

It is:

- visible by default,
- manually collapsible,
- not replaced by a hamburger menu,
- not automatically hidden because the viewport crosses a responsive breakpoint.

The rail includes one labelled search field while expanded. It filters the startup
Workspace inventory by trimmed, case-insensitive workflow-name substring without
changing selection; the same filtered inventory drives compact icons. The query
survives rail collapse/reopen for the page session and resets on reload.

Collapsed state is an in-memory display choice, not a different information architecture. It resets when the page reloads. Arbitrary workflow names do not need invented iconography merely to support a compact mode.

## Center stage rail

The center is the primary work surface and presents stages as an **ordered vertical narrative**, not a DAG.

It must distinguish at least:

- accepted/completed stages,
- the current accepted position,
- an active pending transition,
- future/not-yet-applied stages.

A pending transition must be visually distinct from both accepted and untouched stages. When useful, the pending stage may expose a compact mutation/verification summary inline, for example:

```text
◐ 003 add suffix
   up          ✓
   verify-up   ✕
```

Selecting a stage is **read-only inspection**. It changes the right inspector. It does not execute the stage, change the checkpoint or redefine the next legal transition.

An initial selection may follow relevance:

```text
pending transition exists
  -> select pending stage

otherwise an accepted stage exists
  -> select current accepted stage

otherwise
  -> select the first available stage
```

The exact initial selection is an implementation choice. After a successful forward transition, selection may follow the newly accepted stage. After failure, selection may follow the failed/pending stage. Manual inspection of another stage must not alter execution state.

## Persistent right stage inspector

The right rail is a persistent stage inspector, taking inspiration from Decagon's simultaneous context and Dagu/Inngest's detail surfaces.

It is:

- visible by default,
- independently scrollable,
- resizable from its left edge with a keyboard-accessible vertical separator,
- initialized to 382px when no usable remembered width exists,
- remembered by a width-only localStorage value and clamped to the current desktop geometry,
- manually collapsible,
- collapsed only for the current page session.

Resizing preserves the center's 520px minimum width and the existing horizontal
overflow on narrow desktops. Width survives inspector collapse/reopen and reload;
collapse itself remains session-only. Workflow and stage rows retain their current
rendered spacing. Typography and checkpoint presentation retain the reviewed
current design.

A responsive breakpoint must not transform it into a mobile drawer, bottom sheet or separate navigation flow. If the user deliberately collapses the inspector, merely selecting another stage should not reopen it without an explicit reopen affordance.

Inspector information is ordered by usefulness:

```text
stage state
-> mutation
-> verification
-> relevant output
-> technical stage/executable details
```

For a current-session attempt, mutation and verification are separate first-class results. Verification success/failure must be visible without opening logs.

On failure, error information belongs near the top. stdout and stderr remain distinct because the runner preserves them separately and does not establish cross-stream chronology.

For a future stage, the inspector may preview what advancing would invoke. For an older accepted stage with no durable attempt history, the UI may truthfully show **Applied**; it must not fabricate an old verifier result that is no longer persisted.

Persistent execution history remains deferred. The shell should leave room for richer historical attempt data later without requiring that storage feature in the first UI.

## Declarative progression and recovery controls

Do not center the UI on generic per-stage **Run** buttons.

The primary action region should describe user intent and the resulting state transition, for example:

```text
Advance to 003 · Add suffix
  Runs up, then verify-up
```

or:

```text
Back out 003 -> 002
  Runs down, then verify-down
```

When verification is pending or has failed, progression gives way to the legal recovery choices such as:

```text
Retry verification
Back out 003
```

The UI may explain the mechanical behavior underneath the declarative label, but React/HTTP handlers do not decide which actions are legal. Application remains authoritative for retry, reversal, target reachability and nearest recovery choices.

The action region stays in one predictable location so the user always has a clear answer to **“what can I do next?”**

## Desktop-only constraint

This is an explicit product requirement:

> Do not implement mobile or tablet responsive layouts for the first Control Tower UI.

In particular, a frontend implementation must not automatically transform the three-column workbench into:

- stacked cards,
- hamburger navigation,
- temporary mobile drawers,
- bottom sheets,
- a single-pane drill-down flow.

Manual rail collapse is permitted because it is an explicit user choice. Responsive breakpoints must not change the information architecture.

If necessary, the implementation may enforce a sensible minimum desktop width or preserve horizontal layout rather than redesigning the product for a narrow viewport. Exact pixel widths are implementation tuning, not part of this ADR.

## Visual references

The accepted semantics above are Control Tower-specific. [The UI reference review](../research/2026-10-02-ui-reference-review.md) preserves the source screenshots and the narrower patterns borrowed from Dagu, Inngest, Decagon and Playwright.

The implementation agent should use those references for density, simultaneous visibility and inspector behavior without copying unrelated DAG/orchestration/admin features.

## What this does not decide

This ADR does not select:

- a component library,
- exact colors or typography,
- exact rail widths,
- icon set,
- persistent run/execution history,
- live byte-by-byte stdout/stderr streaming,
- workspace switching,
- mobile/tablet behavior,
- a DAG/graph representation.

Those choices require separate evidence or implementation-level tuning. The desktop shell, however, is not optional interpretation.
