---
id: CT-DOCS
title: Control Tower documentation
type: index
status: maintained
created: '2026-09-30'
updated: '2026-10-02'
owner: christopher-caldwell
authored_by: assistant
sources:
- research/documentation-strategy.md
- history/2026-09-30-initial-design.md#scope-and-provenance
- research/documentation-validation.md
---

# Control Tower documentation

Control Tower is a local workbench for user-owned executable stages. The CLI is the current implemented interface; a loopback browser UI is selected as the first graphical Entry but is not implemented yet. Start with the runnable example; the architecture history is not a setup prerequisite.

## Using Control Tower

| Need | Start here |
| --- | --- |
| Try it immediately | [Root quickstart](../README.md#try-the-three-stage-example) |
| Install/build and prepare storage | [Getting started](guides/getting-started.md) |
| See each stage's effect | [Three-stage UUID-file walkthrough](../examples/simple/workspaces/uuid-file/README.md) |
| Carry an application-generated ID between stages | [Optional two-stage SQLite/JSON example](../examples/simple/workspaces/generated-id/README.md) (Python 3) |
| Choose a usage pattern | [Example gallery](../examples/README.md) |
| Write your own stages | [Create a workspace](guides/creating-a-workspace.md) |
| Retry a check or back out unfinished work | [Navigation and verification](guides/verification-and-navigation.md) |
| Look up commands and database operations | [CLI reference](reference/cli.md) |
| Look up role names, paths and environment variables | [Executable contract](reference/stage-executables.md) |
| Diagnose setup or script failures | [Troubleshooting](guides/troubleshooting.md) |

Guides describe the current executable interface. They do not promise features from earlier design probes. The [documentation validation](research/documentation-validation.md) records the implementation revision checked and separates newly executed checks from previous worker evidence. The [October 2 gallery validation](research/2026-10-02-examples-gallery-validation.md) records the family/workspace restructure and its executed integration checks.

## Design and development

Read [Current design](design/current-design.md) for the maintained architecture and scope. Accepted decisions explain intent; dated research and history explain how it was reached.

| Record | Standing | Scope |
| --- | --- | --- |
| [ADR-0001](decisions/0001-user-owned-executables.md) | Accepted | User-owned, shebang-led executables, not built-in action drivers. |
| [ADR-0002](decisions/0002-stage-navigation-and-verification.md) | Accepted leading mechanism | Ordered up/down and optional directional verification before completion. |
| [ADR-0003](decisions/0003-session-state-and-process-io.md) | Proposed | Minimal identifier handoff; shipped runner-generated UUID and unimplemented output ideas are distinguished. |
| [ADR-0004](decisions/0004-session-storage-port-and-adapters.md) | Accepted | Application-owned storage contracts with SQLite as the v0 runtime adapter. |
| [ADR-0005](decisions/0005-cli-first-driving-adapter.md) | Accepted | CLI driving adapter and explicit composition; future UI stays outside Application. |\n| [ADR-0006](decisions/0006-loopback-web-ui.md) | Accepted | First graphical Entry uses embedded React over loopback HTTP/SSE; Tauri and live byte streaming remain unselected. |

[Playbook compliance](research/playbook-compliance.md) records the architecture correction. [Run-semantics validation](research/run-semantics-validation.md) records the later behavior correction and executed evidence. Neither is a replacement for the accepted decisions or a universal certification of future changes.

[Guided-usage findings and core refinement](research/2026-10-01-guided-usage-findings.md) retains all F01–F15 findings/corrections and distinguishes inherited experiments from newly executed phase acceptance. Deferred entries are retained knowledge, not additional acceptance criteria or an implementation backlog for this round.

[Core-v0 completion and validation](research/2026-10-01-core-v0-completion.md) is the authoritative milestone closeout: bounded standard, independent assessment, audited AR06/AR07 evidence, fresh layered navigation/continuation, examples, gates and deliberately retained limits. Core v0 is ready for repeated owner use. The [scenario registry](research/autonomous-scenario-registry.md) and [cycle summaries](research/autonomous-usage-runs.md) retain research coverage; untried entries do not reopen this completed milestone.

[Open questions](design/open-questions.md) tracks deferred work, not extra first-use prerequisites. The [discovery brief](design/discovery-brief.md), [first discovery record](research/discovery-01.md), [earlier three-step probe](design/three-step-workspace.md) and [initial history](history/2026-09-30-initial-design.md) preserve historical context. Do not use old `steps/` layouts, memory-only assumptions, or checkpoint/patch hypotheses there as current CLI instructions.

## Documentation structure

```text
docs/
  README.md       navigation and maintenance conventions
  guides/         setup, authoring, navigation, troubleshooting
  reference/      implemented CLI and executable contract
  design/         current synthesis and labeled design inputs/probes
  decisions/      accepted intent and explicitly proposed choices
  research/       sourced research and dated validation evidence
  history/        dated discussions and corrections
```

The root README is the short entry point. Each example README belongs beside its runnable scripts. All deeper project guidance stays under `docs/`. There is no generated documentation site or per-folder index to maintain.

## Authority and maintenance

Explicit user choices and accepted decisions govern the product. Guides/reference describe implementation, not new approval. If code and an accepted decision disagree, identify the mismatch; do not rewrite the decision to make code appear correct. Proposals do not become accepted because they were committed or repeated. Preserve historical evidence with its original revision and execution limits, adding dated links to later evidence rather than rewriting a past run.

Update the relevant guide/reference when an interface changes. Update the current design and decision record together only when intent changes. A material documentation or implementation observation can be recorded in one cohesive dated research/validation record; do not create a file per chat response or checklist item.

For internal links, use relative paths and descriptive link text. Keep one canonical reference for commands and one for the executable/environment contract. Short runnable examples may repeat commands, but they must be checked together when syntax changes. Each guide states prerequisites, working directory, expected observations and failure behavior where relevant. Keep credentials, private fixture data, real work identifiers and generated run files out of the public repository.

## Metadata convention

Markdown files under `docs/` use YAML front matter. Root/example READMEs remain plain reader-facing Markdown. Keep metadata small; explanation belongs in the body.

```yaml
id: CT-EXAMPLE
title: A descriptive title
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../decisions/0002-stage-navigation-and-verification.md
```

`id` is unique and stable. `owner` is maintenance responsibility, not approval; `authored_by` is authorship, not decision authority. `sources` contains relevant relative paths or primary-source URLs. Quote date strings; use UTC for event timestamps.

Use `maintained` for index, guide, reference and living design documents. Decision records use `proposed`, `accepted`, `rejected` or `superseded`, with `decision_authority` and a sourced `decision_date` when accepted. Research/history use `recorded`; research additionally states `verified_on` and `method`, and history retains its source window/correlation information. A historical design input remains clearly labeled in its body even when its original metadata is preserved.

Number ADR filenames sequentially and never reuse a number. Keep topic names for maintained guides and ISO dates for history. Before a documentation commit, check relative targets/anchors, unique IDs, front matter, code-block syntax, command/source alignment and the distinction between executed, source-inspected and unavailable verification. See [the organization strategy](research/documentation-strategy.md) and [the latest docs audit](research/documentation-validation.md).
