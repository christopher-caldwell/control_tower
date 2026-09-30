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
---

# Control Tower documentation

Control Tower is a personal, local, migration-style workbench for user-owned executable actions. The developer supplies the work; the tool supplies ordered navigation, execution, results, and a small amount of shared working context.

The project is intentionally not an orchestration platform. No login, hosting service, scheduler, worker fleet, built-in HTTP/database action model, or DAG engine is part of the current identity.

**This is a design baseline, not an implemented product or an approved implementation specification.** Explicitly accepted decisions are called out separately from proposals and design probes.

## Start here

Read [Current design](design/current-design.md) for the distilled direction. Then read [Three-step workspace design probe](design/three-step-workspace.md) for the smallest concrete example we have used to challenge it. [Open questions and validation](design/open-questions.md) contains what still needs to earn acceptance. The [initial discussion history](history/2026-09-30-initial-design.md) records how the direction changed.

## Structure and responsibilities

~~~text
docs/
  README.md
  design/
    current-design.md
    three-step-workspace.md
    open-questions.md
  decisions/
    0001-user-owned-executables.md
    0002-stage-navigation-and-verification.md
    0003-session-state-and-process-io.md
  research/
    documentation-strategy.md
    existing-tools.md
  history/
    2026-09-30-initial-design.md
~~~

| Location | Question it answers | What belongs here |
| --- | --- | --- |
| design/ | What are we designing now? | The living synthesis, concrete design probes, and unresolved questions. |
| decisions/ | Why choose this rather than an alternative? | One significant decision per numbered record, including drawbacks and validation. |
| research/ | What evidence supports or challenges the design? | Dated, sourced observations, separated from our interpretation. |
| history/ | What was said or changed, and when? | Dated discussion summaries and explicit corrections. |

Do not create one file per chat response, feature idea, or research link. Add to the existing home for a subject unless it has an independently maintainable purpose. The three-step probe has its own file because it is a reusable concrete model for testing multiple design claims, not because every example deserves a document.

This structure combines reader-oriented separation with lightweight decision records. The alternatives and sources are in [Documentation strategy](research/documentation-strategy.md).

## Decision index

| Record | Status | Scope |
| --- | --- | --- |
| [ADR-0001](decisions/0001-user-owned-executables.md) | Accepted | User-owned, shebang-led executable authoring; not a driver platform. |
| [ADR-0002](decisions/0002-stage-navigation-and-verification.md) | Accepted core; directional verification under evaluation | Ordered migration-style transitions with verified forward completion; optional verify-down is the current design candidate. |
| [ADR-0003](decisions/0003-session-state-and-process-io.md) | Proposed; leading candidate | Workbench-owned context, process input snapshot, and a separate machine-output channel. |

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

Use sequential four-digit ADR filenames. Never reuse a number. Use topic names for living documents and ISO dates for history. When an accepted decision changes, add a replacement record and link the old and new records using supersedes / superseded_by; do not silently rewrite the old rationale. A proposed ADR can evolve before acceptance.

## Authority and maintenance

Explicit user choices, with traceable sources, govern this design. Accepted ADRs summarize those choices; proposals and research do not override them. The current design is a navigation aid and synthesis, not a second competing source of approval. Conflicts require an explicit correction or decision, not an assistant selecting whichever text is newest.

Update the relevant current-design section and decision record together when a choice changes. Append a history entry when the change is significant. Keep external product claims in research and link to them instead of duplicating a feature matrix throughout the docs. Clearly label illustrative filesystem layouts and protocols until accepted.

Before committing documentation, check front matter, unique IDs, relative links, status consistency, source attribution, and whether new claims were actually approved. Keep credentials, real fixture data, private work details, and runtime output out of this public repository.
