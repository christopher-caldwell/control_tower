---
id: CT-DOCS
title: Control Tower documentation
type: index
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- research/documentation-strategy.md
- history/2026-09-30-initial-design.md#scope-and-provenance
- research/discovery-01.md
---

# Control Tower documentation

Control Tower is a personal, local, migration-style workbench for user-owned executable actions. The developer supplies the work; the tool supplies ordered navigation, execution, results, and a small amount of shared working context.

The project is intentionally not an orchestration platform. No login, hosting service, scheduler, worker fleet, built-in HTTP/database action model, or DAG engine is part of the current identity.

**This is a design baseline, not an implemented product or an approved implementation specification.** Explicitly accepted decisions are called out separately from proposals and design probes.

## Start here

Read [Current design](design/current-design.md) for the distilled direction and [First formal discovery brief](design/discovery-brief.md) for the discovery seed. The first collaborative discovery is now in progress; its inspected revisions, findings, and evidence limits are in [First discovery record](research/discovery-01.md).

[Open questions and validation](design/open-questions.md) tracks the current product decision. [Three-step workspace design probe](design/three-step-workspace.md) preserves earlier conceptual pressure tests, not additional v0 obligations. The [initial discussion history](history/2026-09-30-initial-design.md) records how the direction changed.

## Structure and responsibilities

~~~text
docs/
  README.md
  design/
    current-design.md
    discovery-brief.md
    three-step-workspace.md
    open-questions.md
  decisions/
    0001-user-owned-executables.md
    0002-stage-navigation-and-verification.md
    0003-session-state-and-process-io.md
    0004-session-storage-port-and-adapters.md
    0005-cli-first-driving-adapter.md
  research/
    documentation-strategy.md
    existing-tools.md
    discovery-01.md
  history/
    2026-09-30-initial-design.md
~~~

| Location | Question it answers | What belongs here |
| --- | --- | --- |
| design/ | What are we designing now? | The living synthesis, concrete design probes, and unresolved questions. |
| decisions/ | Why choose this rather than an alternative? | One significant decision per numbered record, including drawbacks and validation. |
| research/ | What evidence supports or challenges the design? | Dated, sourced observations, separated from our interpretation. |
| history/ | What was said or changed, and when? | Dated discussion summaries and explicit corrections. |

Do not create one file per chat response, feature idea, or research link. Add to the existing home for a subject unless it has an independently maintainable purpose. The discovery record is one accumulating investigation record, not a separate specification or a claim of independent repeated runs.

## Decision index

| Record | Status | Scope |
| --- | --- | --- |
| [ADR-0001](decisions/0001-user-owned-executables.md) | Accepted | User-owned, shebang-led executable authoring; not a driver platform. |
| [ADR-0002](decisions/0002-stage-navigation-and-verification.md) | Accepted leading mechanism, subject to discovery | Ordered up/down transitions with optional verify-up and verify-down before completion. |
| [ADR-0003](decisions/0003-session-state-and-process-io.md) | Proposed; implementation choice | Minimal opaque UUID handoff for the first demonstration; richer checkpoint/patch mechanics are not v0 requirements. |
| [ADR-0004](decisions/0004-session-storage-port-and-adapters.md) | Accepted | Core/application owns the session-state storage port; SQLite is the v0 runtime adapter; memory may remain useful as a test adapter. |
| [ADR-0005](decisions/0005-cli-first-driving-adapter.md) | Accepted | CLI is the only v0 driving adapter and composition edge; future Tauri/HTTP entry adapters call the same application use cases. |

The status of a document is not evidence that its design is implemented. A proposal does not become accepted because it was committed to main or because an assistant repeated it.

## Metadata convention

Every Markdown document has YAML front matter. Metadata describes the document; the body contains its explanation and reasoning. Use this small common set:

~~~yaml
id: CT-EXAMPLE
title: A descriptive title
type: design
status: maintained
created: "2026-09-30"
updated: "2026-09-30"
owner: christopher-caldwell
authored_by: assistant
sources:
  - ../history/2026-09-30-initial-design.md#scope-and-provenance
~~~

id is unique and stable. owner identifies maintenance responsibility, not approval. authored_by identifies who wrote the synthesis, not who made the underlying choices. sources contains relative document links or external URLs; link to the smallest relevant section where possible. Quote date strings. Use UTC for recorded event timestamps.

Decision records additionally use decision_authority; accepted records include decision_date and a source for that acceptance. Research uses verified_on and method. History uses source_window and a human-readable correlation_id. Add metadata only when it answers a real retrieval or provenance question.

Statuses are deliberately type-specific:

- Decisions: proposed, accepted, rejected, or superseded.
- Index and design: maintained; individual statements still distinguish chosen direction, candidate, and open question.
- Research and history: recorded; this means captured, not universally correct or authoritative.

Use sequential four-digit ADR filenames. Never reuse a number. Use topic names for living documents and ISO dates for history.

## Authority and maintenance

Explicit user choices, with traceable sources, govern this design. Accepted ADRs summarize those choices; proposals and research do not override them.

Update the relevant current-design section and decision record together when a choice changes. Append a history entry when the change is significant. Keep credentials, real fixture data, private work details, and runtime output out of this public repository.
