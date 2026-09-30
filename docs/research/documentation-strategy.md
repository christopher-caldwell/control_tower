---
id: CT-RESEARCH-DOCS
title: Documentation organization strategy
type: research
status: recorded
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
verified_on: '2026-09-30'
method: Primary-source review and project-specific evaluation
sources:
- https://diataxis.fr/
- https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
- https://adr.github.io/madr/
- https://adr.github.io/madr/decisions/0013-use-yaml-front-matter-for-meta-data.html
---

# Documentation organization strategy

## The problem to solve

The owner requested all project documentation under `docs/`, a deliberate folder structure, decision/timeline capture, and metadata in front matter. The danger is not too few documents: it is accumulated chat summaries with no clear distinction between current intent, historical speculation, external evidence, and approved decisions.

The structure was selected for this documentation task. It does not select application architecture or impose an implementation workflow.

## Strategies examined

**Reader-oriented documentation.** [Diátaxis](https://diataxis.fr/) separates tutorials, how-to guides, reference, and explanation by reader need. Borrow the separation principle. Do not create all four folders now: there is no implemented tool to teach or reference. Current design explanation and research are the immediate needs. Add how-to/reference content only once there is a real interface to describe.

**Lightweight architecture decision records.** [Michael Nygard's ADR approach](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) uses small records with context, decision, status, and consequences. It retains superseded decisions and numbers records monotonically. Borrow the rationale and lifecycle. A chronological chat dump cannot substitute for an explicit decision, and a current-design page should not have to retell every rejected alternative.

**Structured Markdown and metadata.** [MADR](https://adr.github.io/madr/) provides a compact decision format with alternatives and confirmation, and its [front-matter decision](https://adr.github.io/madr/decisions/0013-use-yaml-front-matter-for-meta-data.html) separates metadata from content while acknowledging portability and false-precision costs. Borrow a small YAML header, a clear owner/author distinction, and explicit approval status. Do not import a full governance template, RACI matrix, site generator, or ADR toolchain.

**Chronological design journal.** For this project, a dated synthesis is useful evidence of how the conversation evolved. It is not a separate external methodology adopted here. Store the original chronology and corrections, but make the current design the starting point for readers. This avoids forcing future work to infer current intent from the last enthusiastic assistant message.

## Chosen separation

Use four content homes: `design/` for the current model and open questions; `decisions/` for reasons and acceptance; `research/` for sourced observations; `history/` for chronology. `docs/README.md` is the only top-level index and documentation convention. No separate index is needed in each small folder.

The operational convention and exact metadata fields live in [the documentation index](../README.md), not here. This page records why those choices were made; it is not a second policy document.

## Alternatives rejected for now

A single large design document has low startup cost but mixes explanations with decisions and history. A file per topic fragment makes navigation and authority harder than reading the original chat. A full RFC process creates review bureaucracy disproportionate to a personal project. A generated documentation website adds tooling without improving the present source-of-truth problem.

The proposed compromise is nine linked documents with different maintenance lifecycles, not nine independent specifications. Current design and open questions change as the project learns. Accepted decisions retain their rationale. Research retains dates and source limits. History is corrected explicitly rather than rewritten into retroactive agreement.

## Evaluation

The structure earns its place if a future reader can answer: what is chosen, what remains provisional, why a choice was made, where the evidence is, and when the owner actually supported it. A large amount of front matter is not itself a quality goal; missing approval must remain missing rather than being filled with an invented date or confident status.
