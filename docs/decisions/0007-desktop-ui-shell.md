---
id: ADR-0007
title: Use a desktop-only three-column workbench shell
type: decision
status: accepted
created: '2026-10-02'
updated: '2026-10-02'
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

The first graphical Control Tower interface is a **desktop-only macOS developer workbench**.

Its primary workspace screen uses three simultaneously visible regions:

```text
┌────────────────┬──────────────────────────────┬─────────────────────────┐
│ WORKSPACES     │ STAGES                       │ STAGE INSPECTOR         │
│                │                              │                         │
│ left rail      │ primary execution context    │ persistent right rail   │
│                │                              │                         │
└────────────────┴──────────────────────────────┴─────────────────────────┘
                 sticky declarative action region
```

The shell deliberately preserves context. It is not a mobile-first or responsive content site.

## Terminology and launch scope

Use these terms consistently:

- **Project**: the enclosing developer project/environment. In the examples repository, `simple`, `multi_language` and `py_capsule` are projects.
- **Workspace**: a runnable Control Tower scratch/work area inside a project, with its own stages and checkpoint state.
- **Stage**: one ordered migration-style transition within a workspace.

For UI v0, the directory from which the UI is launched establishes the **Project**. Control Tower discovers workspaces from that project context and the user may switch among those workspaces in the UI.

Project switching is deferred. Do not introduce a global project registry, recent-project list, favorites, moved-path recovery or an “Open project” product model for v0. If repeated real use makes relaunching from another project materially painful, project switching can be reconsidered.

The exact filesystem algorithm for identifying workspace candidates is an implementation detail to resolve against existing project/workspace conventions; this ADR does not authorize a new workspace manifest or global registry.

## Left workspace rail

The left rail is the project-local workspace navigator.

It is:

- visible by default,
- manually collapsible,
- not replaced by a hamburger menu,
- not automatically hidden because the viewport crosses a responsive breakpoint.

Collapsed state is a space preference, not a different information architecture. Arbitrary workspace names do not need invented iconography merely to support a compact mode.

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

Initial selection should follow relevance:

```text
pending transition exists
  -> select pending stage

otherwise an accepted stage exists
  -> select current accepted stage

otherwise
  -> select the first available stage
```

After a successful forward transition, selection may follow the newly accepted stage. After failure, selection remains on the failed/pending stage. Manual inspection of another stage must not alter execution state.

## Persistent right stage inspector

The right rail is a persistent stage inspector, taking inspiration from Decagon's simultaneous context and Dagu/Inngest's detail surfaces.

It is:

- visible by default,
- independently scrollable,
- user-resizable,
- manually collapsible,
- allowed to remember its width and collapsed state as local UI preference.

A responsive breakpoint must not transform it into a mobile drawer, bottom sheet or separate navigation flow. If the user deliberately collapses the inspector, merely selecting another stage should not override that preference without an explicit reopen affordance.

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
- project switching,
- mobile/tablet behavior,
- a DAG/graph representation.

Those choices require separate evidence or implementation-level tuning. The desktop shell, however, is not optional interpretation.
