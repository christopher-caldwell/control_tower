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

Control Tower is being explored as a personal, local, Rust-based control surface for user-owned executable actions. The developer supplies the work; the tool supplies navigation, results, and shared working context.

**This is a design baseline, not an implemented product or an approved implementation specification.** The source repository contained only its initial README when this documentation was prepared. No application code is introduced by this documentation set.

## Start here

Read [Current design](design/current-design.md) for the distilled proposal, then [Open questions and validation](design/open-questions.md) for what needs to earn acceptance next. The [initial discussion history](history/2026-09-30-initial-design.md) records how the direction changed without making every earlier suggestion a requirement.

## Structure and responsibilities

```text
 docs/
   README.md
   design/
     current-design.md
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
```

| Location | Question it answers | What belongs here |
| --- | --- | --- |
| `design/` | What are we designing now? | The living synthesis and unresolved decisions. |
| `decisions/` | Why choose this rather than an alternative? | One significant decision per numbered record, including drawbacks and validation. |
| `research/` | What evidence supports or challenges the design? | Dated, sourced observations, separated from our interpretation. |
| `history/` | What was said or changed, and when? | Dated discussion summaries and explicit corrections. |

Do not create one file per chat response, feature idea, or research link. Add to the existing home for a subject unless it has an independently maintainable purpose. Do not create empty tutorial, API-reference, implementation-plan, or component directories in anticipation of a product that does not exist.

This structure combines reader-oriented separation with lightweight decision records. The alternatives and sources are in [Documentation strategy](research/documentation-strategy.md).

## Decision index

| Record | Status | Scope |
| --- | --- | --- |
| [ADR-0001](decisions/0001-user-owned-executables.md) | Accepted direction | User-owned, shebang-led executable authoring; not a driver platform. |
| [ADR-0002](decisions/0002-stage-navigation-and-verification.md) | Proposed | A precise interpretation of the user's up/down and assertion ideas. |
| [ADR-0003](decisions/0003-session-state-and-process-io.md) | Proposed; leading candidate | Workbench-owned context, process input snapshot, separate proposed outputs. |

The status of a document is not evidence that its design is implemented. A proposal does not become accepted because it was committed to `main` or because an assistant repeated it.

## Metadata convention

Every Markdown document has YAML front matter. Metadata describes the document; the body contains its explanation and reasoning. Use this small common set:

```yaml
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
```

`id` is unique and stable. `owner` identifies maintenance responsibility, not approval. `authored_by` identifies who wrote the synthesis, not who made the underlying choices. `sources` contains relative document links or external URLs; link to the smallest relevant section where possible. Quote date strings. Use UTC for recorded event timestamps rather than inferring a local date from a chat title.

Decision records additionally use `decision_authority`; accepted records include `decision_date` and a source for that acceptance. Research uses `verified_on` and `method`. History uses `source_window` and a human-readable `correlation_id`. Add metadata only when it answers a real retrieval or provenance question; do not duplicate the body as a large YAML database.

Statuses are deliberately type-specific:

- Decisions: `proposed`, `accepted`, `rejected`, or `superseded`.
- Index and design: `maintained`; individual statements still distinguish chosen direction, candidate, and open question.
- Research and history: `recorded`; this means captured, not universally correct or authoritative.

Use sequential four-digit ADR filenames. Never reuse a number. Use topic names for living documents and ISO dates for history. When an accepted decision changes, add a replacement record and link the old and new records using `supersedes` / `superseded_by`; do not silently rewrite the old rationale. A proposed ADR can evolve before acceptance.

## Authority and maintenance

Explicit user choices, with traceable sources, govern this design. Accepted ADRs summarize those choices; proposals and research do not override them. The current design is a navigation aid and synthesis, not a second competing source of approval. Conflicts require an explicit correction or decision, not an assistant selecting whichever text is newest.

Update the relevant current-design section and decision record together when a choice changes. Append a history entry when the change is significant. Keep external product claims in research and link to them instead of duplicating a feature matrix throughout the docs. Clearly label examples as illustrative until a protocol is accepted.

Before committing documentation, check front matter, unique IDs, relative links, status consistency, source attribution, and whether new claims were actually approved. Keep credentials, real fixture data, private work details, and runtime output out of this public repository. An implementation checklist or automated documentation site is not required yet.
