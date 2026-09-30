---
id: CT-RESEARCH-DISCOVERY-01
title: First collaborative discovery — source review and findings
type: research
status: recorded
discovery_status: in-progress
created: '2026-09-30'
updated: '2026-09-30'
verified_on: '2026-09-30'
started_at: '2026-09-30T22:21:04Z'
owner: christopher-caldwell
authored_by: assistant
method: Remote repository and documentation inspection; conceptual scenario analysis
baseline_commit: 4e22c7fcaa0893e6479163ab77acf48ab88ed620
playbook_revision: 8d2ff6d906676020d671ba3b0674c0f8b58d9414
chat_discovery_revision: f20c308d6274ee23a67a7c0e07f72541be49b454
sources:
- ../design/discovery-brief.md
- ../design/open-questions.md
- https://github.com/christopher-caldwell/ochestration/blob/f20c308d6274ee23a67a7c0e07f72541be49b454/docs/chat-discovery.md
- https://github.com/christopher-caldwell/rust_simpler_playbook/tree/8d2ff6d906676020d671ba3b0674c0f8b58d9414
---

# First collaborative discovery — source review and findings

## Status and provenance

The owner requested discovery to begin at 22:21:04Z on September 30, 2026 and supplied the Rust playbook and Chat Discovery guide. This is one ongoing collaborative Discovery source, not several independent runs. This file will accumulate findings from that source rather than creating a file per reply.

The inspected Control Tower baseline is `main` at `4e22c7fcaa0893e6479163ab77acf48ab88ed620`. Its tree contains the root README and thirteen Markdown documents under `docs/`; no application implementation, Cargo manifest, or executable fixture was present.

This pass read repository content through the GitHub connector. It did not inspect a local developer checkout, execute the UUID fixture, run Rust tests, import a Chat Discovery bundle, or launch Build. Behavioral traces below are reasoning about proposed behavior, not observed runtime results.

The source revisions above identify this investigation's inputs. They do not claim that an Orchestrate effort or frozen request has been created. A future import uses its actual committed repository HEAD under the import procedure.

## Authority handling

The [Chat Discovery guide](https://github.com/christopher-caldwell/ochestration/blob/f20c308d6274ee23a67a7c0e07f72541be49b454/docs/chat-discovery.md) distinguishes implementation obligations, advisory suggestions, and Build planning. Both explicit-user and chat-analysis entries in a future `requirements` array become binding. Therefore implementation experiments, proposed storage representations, and historical assistant suggestions must not be promoted into that array merely to preserve discussion detail.

The owner supplied the product boundary: local Rust workbench; CLI-only v0; user-owned executable stages; optional directional verification; filesystem-first authoring; SQLite behind an inward-owned storage port. The owner also excluded concurrency protection, directory-drift protection, crash reconciliation, hosted services, UI work, and a helper ecosystem from this first version.

The four executable roles are the accepted leading mechanism, subject to discovery. Exact filenames, CLI flags, Rust traits, context transport, and database layout remain design choices. The UUID/file scenario is a requested demonstration, not permission to make Control Tower itself a UUID-file-specific engine.

No portable handoff is produced during this pass. A later explicit handoff request triggers packaging and the guide's authority/acceptance review. Build remains a separate authorization.

## Scoped Rust playbook review

The playbook was read at the revision above, rather than inferred from earlier assistant summaries. Its [precedence rules](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/precedence.md) distinguish hard boundaries, defaults, conditional concerns, and open questions. Known problems are not permission to prebuild their solutions.

| Concern | Source reviewed | Application to this project | Evidence now |
| --- | --- | --- | --- |
| Workspace boundaries | [Workspace rules](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/workspace.md) and the owning workspace decision through its Database-boundary section | Preserve inward dependency boundaries. `core` in earlier diagrams is conceptual, not a prescribed package. Add only actual concerns; authentication and HTTP are absent. | Documentation mapping only; no Cargo graph exists yet. |
| Use cases and ports | [Services](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/application-services.md) and [ports](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/ports.md) | Application owns actual use cases and capability contracts. SQLite and process execution stay outside. Do not invent generic CRUD or a service locator. | Existing intent agrees; implementation not exercised. |
| Composition | [Composition rules](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/composition.md) | The CLI executable assembles its object graph and resolves external configuration. Terminal/protocol types do not become application inputs. | Source review only. |
| Persistence | [Lane selector](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/persistence.md) | Classify concrete reads/writes by semantics when methods exist. `SessionStateStore` is a placeholder, not a mandate for a generic Store or UnitOfWork. | No methods or schema selected. |
| Database setup | [Migration rules](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/migrations.md) and the owning bootstrap decision through its Migration Execution section | Distinguish Control Tower's own schema setup from the user's arbitrary up/down scripts. Keep setup out of application use cases. Scope the source's PostgreSQL-specific operational examples to the actual SQLite adapter rather than copying them as universal product requirements. | Setup approach still to be worked out; no environment proof claimed. |
| Verification evidence | [Testing rules](https://github.com/christopher-caldwell/rust_simpler_playbook/blob/8d2ff6d906676020d671ba3b0674c0f8b58d9414/docs/implementation/rules/testing.md) | Eventually exercise the real SQLite/process boundaries and distinguish those results from fake-based application tests. No test framework is dictated. | No application tests run in this pass. |

This is an applicability review, not a blanket playbook-compliance claim. Implementation-specific rules need their focused source sections and actual evidence when implementation begins.

## Findings

### F1 — Some repository text lagged behind the latest user choices

At the baseline, ADR-0002 and the earlier three-step probe still described memory-only session lifetime. The index still described verify-down as awaiting acceptance, and the probe still included a general auxiliary-action directory. The current discovery brief instead selected SQLite v0, a UUID-only demonstration, and no auxiliary-action abstraction.

These are documentation inconsistencies, not reasons to reopen the owner's decisions. The earlier probe is preserved as historical analysis; its checkpoint/patch design is not a v0 requirement. The ADR session-scope text and index are corrected to the current SQLite decision.

### F2 — There are two different kinds of state

The UUID-named file is the script-controlled fixture. The stage position and identifier remembered between CLI invocations are Control Tower metadata stored through the SQLite adapter.

For the demo, the scripts change the file and verifiers inspect it. Control Tower need only carry the opaque ID and remember enough execution outcome to support the requested navigation. It does not need to understand `hello`, take snapshots of file contents, or prescribe a general context-patch language.

The fixture phrase “without a database” means no database-backed test application is required. It does not remove SQLite from Control Tower itself.

### F3 — A returned script failure is not a Rust-process crash

If `03/up` exits successfully but `03/verify-up` fails, the CLI must be able to report that result, exit normally, and let a later invocation retry verification against the same UUID without rerunning `03/up`. Otherwise SQLite would not deliver the requested iterative CLI experience.

The exclusion of crash recovery concerns a crashed Control Tower process. It is not a reason to discard ordinary child-process failures or verification results. This distinction adds no crash journal or external repair mechanism.

### F4 — One ordinary failure path is still a product question

The existing model explicitly covers successful mutation followed by failed verification. Its failed-mutation language says to stop without completing the step; it does not clearly say whether the user may then invoke that same step's down operation.

Concrete trace, not an executed test:

```text
02 completed; UUID file contains "hello"
03/up writes " to you", then exits 1
03/verify-up does not run
02 remains the last completed step

Can the developer explicitly invoke 03/down now?
```

This is different from recovering a crashed Rust process or automatically determining whether the mutation worked.

**Recommendation, pending owner decision:** allow the explicit matching down operation using the already-known UUID, followed by optional verify-down. Do not automatically invoke cleanup, mark the failed up successful, rerun it, or try to reconstruct an identifier that was never captured. This keeps the author in control while preserving the familiar back-out action.

A narrower v0 that leaves cleanup after a nonzero mutation entirely to a terminal is also possible. The tradeoff is an interrupted workbench loop. B1 in the [decision queue](../design/open-questions.md#b1--manual-down-after-up-itself-fails) asks the owner to choose; no new recovery requirement is accepted here.

### F5 — Keep mutations and checks separate in the fixture

The proposed interpretation of the owner's reversal example is: `down` deletes, empties, or removes the suffix; `verify-down` checks that the result is absent, empty, or back to `hello`. This follows the four-role distinction and avoids a verifier changing the thing it claims to observe.

This is an interpretation of the example's intent, not a claim that the owner literally used these role names consistently. It does not imply sandboxing arbitrary scripts or statically proving that a verifier is read-only.

## Proposed acceptance exercise

These are prospective checks, not test results or a frozen requirement set. Exact CLI spellings remain open.

| Exercise | Observable result to check |
| --- | --- |
| Walk 0 -> 1 -> 2 -> 3 | The same UUID names the file throughout; expected contents progress from empty to `hello` to `hello to you`. Each supplied directional check gates completion. |
| Exit and invoke the CLI again between movements | The workspace's recorded position and UUID remain available through the actual SQLite adapter. No long-lived server is required. |
| Walk 3 -> 2 -> 1 -> 0 | Stage 3 removes only its suffix, stage 2 empties the remaining file, stage 1 deletes it. Each verify-down observes its corresponding result. |
| Successful up, failed verify-up | The target is not completed; a later explicit verifier retry does not rerun the mutation. Backing out uses the same stage's down. |
| Successful down, failed verify-down | The lower position is not prematurely recorded as complete; a later verifier retry does not rerun down. |
| Omit a verifier | Successful execution completes the direction without requiring a placeholder verifier. |
| A normal script exits nonzero | The result is visible and no later stage runs. Matching-down behavior remains B1. |

An implementation should exercise the actual scripts and SQLite path, not only simulate this table. These tests have not been run because no implementation exists in the reviewed baseline.

## Next collaborative decision

Resolve B1 only: should explicit same-stage down remain available after up itself returns failure? Continue independent discovery afterward. Do not package unresolved implementation authority or silently substitute the recommendation for the owner's answer.
