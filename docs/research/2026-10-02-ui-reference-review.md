---
id: CT-RESEARCH-2026-10-02-UI-REFERENCES
title: UI reference review for the first graphical workbench
type: research
status: recorded
created: '2026-10-02'
updated: '2026-10-02'
owner: christopher-caldwell
authored_by: assistant
verified_on: '2026-10-02'
method: source-doc-reading-source-code-inspection-and-public-screenshot-review
sources:
- https://docs.dagu.sh/overview/web-ui
- https://github.com/dagucloud/dagu/blob/3cce8be87f2ad9b205105ecdef90a527fc70cb63/ui/src/features/dags/components/step-details/StepDetailsDrawer.tsx
- https://www.inngest.com/docs/platform-and-operations/traces
- https://github.com/inngest/inngest/blob/56f520014f6ed82d1f0630e213a8a2d35eeca64a/ui/packages/components/src/RunDetailsV4/RunDetailsV4.tsx
- https://decagon.ai/blog/decagon-trace-view
- https://www.linkedin.com/posts/decagon-ai_ai-agents-shouldnt-be-black-boxes-as-ai-activity-7405293449354203136-xMhl
- https://playwright.dev/docs/test-ui-mode
---

# UI reference review for the first graphical workbench

This record preserves the source material used while selecting Control Tower's first UI shell. It is **reference evidence, not the product specification**. [ADR-0007](../decisions/0007-desktop-ui-shell.md) records the accepted Control Tower behavior.

The screenshots below remain source-owned public assets and are linked from their publishers rather than copied into this repository.

## Dagu: dense run status and step inspection

![Dagu execution details showing overall run status, workflow steps, status and stdout/stderr actions](https://docs.dagu.sh/status-details.png)

Source: [Dagu Web UI](https://docs.dagu.sh/overview/web-ui).

Useful patterns:

- Overall execution state and individual step state stay visible together.
- Step rows expose stdout and stderr directly rather than hiding output behind a generic log concept.
- The current Dagu step-details implementation uses a right-side drawer with runtime status, errors, timing, separate stdout/stderr actions and a user-resizable width stored in browser local storage.
- The drawer defaults to 560px and is constrained between 420px and 960px in the inspected source revision.

Control Tower should borrow the density, explicit output affordances and resizable inspection surface. It should **not** copy Dagu's DAG semantics, broad operations/admin navigation or graph-first presentation.

## Inngest: persistent selection plus detail context

![Inngest trace view showing the execution timeline beside a selected-step details panel](https://www.inngest.com/assets/docs/platform/monitor/traces/trace-overview.webp)

Source: [Inngest Traces](https://www.inngest.com/docs/platform-and-operations/traces).

Useful patterns:

- The trace screen keeps execution context and selected-step detail visible at the same time.
- The source documentation explicitly defines a left timeline panel and a right details panel with a draggable divider.
- Selecting a step changes inspection context; it does not implicitly execute that step.
- Failed-step details prioritize the error while still retaining timing, input/output and metadata.

Control Tower should borrow the persistent inspector relationship and resizable split. Its center rail remains an ordered migration-style stage narrative rather than a timing waterfall.

## Decagon: readable vertical execution narrative

![Decagon Trace View showing a step-by-step execution narrative](https://media.licdn.com/dms/image/v2/D5622AQGXCTz6scExxQ/feedshare-shrink_800/B56ZsTjJndGgAg-/0/1765559540720?e=2147483647&t=funUdLn6JxiKOWE8NTdgVY6Zh6H4HJ-xkzZC1YMVEOc&v=beta)

Sources: [Decagon Trace View](https://decagon.ai/blog/decagon-trace-view) and Decagon's [public Trace View post](https://www.linkedin.com/posts/decagon-ai_ai-agents-shouldnt-be-black-boxes-as-ai-activity-7405293449354203136-xMhl).

Useful patterns:

- Execution is presented as a coherent sequence rather than a raw log dump.
- Inputs, actions, outputs and transitions are readable in the context of the larger execution.
- The design keeps substantial simultaneous information visible instead of forcing repeated navigation through nested pages or collapsed menus.

This is the strongest visual reference for Control Tower's center stage rail and persistent right inspector.

## Playwright: inspect a step without conflating selection and execution

![Playwright UI Mode showing a selected test action and debugging context](https://playwright.dev/assets/images/ui-mode-1958baf0398aef5e9c9b5c68c5d56f2d.png)

Source: [Playwright UI Mode](https://playwright.dev/docs/test-ui-mode).

Useful pattern: a selected action is something to inspect. Execution controls remain separate. That distinction maps directly to Control Tower's rule that clicking a stage changes the inspector but does not change the legal next transition.

## Control Tower synthesis

The references converge on one useful desktop structure:

```text
project context
┌────────────────┬──────────────────────────────┬─────────────────────────┐
│ WORKSPACES     │ STAGES                       │ STAGE INSPECTOR         │
│                │                              │                         │
│ selected       │ accepted / pending / future  │ status                  │
│ workspace      │ ordered vertical narrative   │ mutation                │
│                │                              │ verification            │
│                │                              │ stdout / stderr          │
│                │                              │ technical details       │
└────────────────┴──────────────────────────────┴─────────────────────────┘
                 sticky declarative progression / recovery controls
```

The borrowed principle is **simultaneous context**. The product-specific semantics remain Control Tower's: project -> workspace -> ordered stage, accepted position distinct from a pending transition, and Application-owned legal next actions.
