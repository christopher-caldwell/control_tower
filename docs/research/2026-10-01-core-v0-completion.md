---
id: CT-CORE-V0-COMPLETION
title: Core-v0 completion and validation
type: research
status: recorded
core_v0_status: complete
created: '2026-10-01'
updated: '2026-10-01'
verified_on: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
method: independent-source-review-inherited-receipt-audit-real-process-layered-navigation-literal-examples-workspace-gates
sources:
- ../design/current-design.md
- ../design/open-questions.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- 2026-10-01-guided-usage-findings.md
- autonomous-scenario-registry.md
- autonomous-usage-runs.md
- ../../crates/application/src/lib.rs
- ../../crates/application/tests/run_semantics.rs
- ../../crates/cli/tests/workbench_cli.rs
---

# Core-v0 completion and validation

**Core v0 is complete as a candidate for repeated local owner use. No additional product implementation is justified by this finishing review.** This is a bounded milestone for the executable/navigation contract below, not a compatibility release, production-ticket certification, or resolution of every retained finding. Repeated real owner usage now has higher value than another autonomous edge-case queue.

## Independent assessment and authority

The starting checkout was `test/workflows` at **`265623137e3fdcac4de34b080439b340277fe93e`**, exactly the handoff head. `main` remained **`c502fda5edb5d5bbfad1f194bff540ec22b459d0`**. The three described candidate commits were present: `03478f5` (feedback/checkpoint truth), `3ec3039` (generated-ID example), and `2656231` (documentation/findings). The only local additions were the two autonomous research records; their pre-existing contents were preserved and committed separately before this closeout.

Review covered the root/index, maintained design/questions, every accepted/proposed ADR, current guides/references, research/history authority, all four packages and tests, both examples, and the full baseline-to-candidate diff. Source supports the accepted navigation decision: Application resolves an active transition before the ordinary walk, distinguishes accepted position from pending mutation, and publishes checkpoints only after successful writes. Database maps/stores; Infrastructure discovers/executes; CLI composes and presents. No architectural refactor or separate Domain package is warranted by these capabilities.

AR06/AR07 were examined through their actual scenario source, reports/corrections, raw command streams, traces and copied SQLite snapshots. A fresh **read-only audit of inherited evidence** cross-checked snapshot summaries against database bytes. AR06 really acknowledged `after` while retaining `before`; AR07 really committed `incorrect`, retried without another mutation, changed only application source, then used the changed source to compensate and reapply. Earlier fixture IDs/results survived. These are inherited scenario executions, not fresh repetitions in this round. AR07's per-request source loading is author-owned fixture behavior; it supplies no Control Tower reload guarantee.

The prompt's A/B/C suggestions share one remaining usage question: do meaningful, layered fixture states remain understandable across farther destinations and pending resolution? They were combined into AR08 rather than treated as three separate projects. The downward variation adds useful proof of preserving an earlier mutation while checking a later compensation. AR07 already proves the source-edit loop strongly enough; replay adds little. AR04's dependency/venv error and a separate AR05 runtime variety exercise are unnecessary for this milestone. The actual owner ticket was not supplied; production infrastructure fidelity would not close that gap. Owner usage is the next phase.

ADR-0003 stays **proposed**. Its original stage-produced-UUID wording and the shipped runner-generated UUID differ; that discrepancy was already recorded and remains explicit. The supplied application-generated-ID need is met by ordinary author-owned JSON, demonstrated by the example and AR06/AR07. This milestone does not accept a managed output/context proposal. No accepted decision was changed, no new semantics were selected, and deferred findings were not promoted to requirements.

## Bounded completion standard

| Requirement | Evidence sufficient for this milestone |
| --- | --- |
| Ordered destination navigation and adjacent compensation persist across normal CLI exits. | Fresh workspace tests, literal UUID-file walkthrough, AR08 meaningful route and two run UUIDs; inherited sparse/native/copy scenarios. |
| Supplied independent verification gates acceptance; process success does not imply correct external state. | Fresh pending-check tests; audited AR06 HTTP acknowledgement and AR07 incorrect persisted mutation. |
| Pending continuation retries only verification; reversal runs the same stage's compensation; farther work waits for resolution. | Fresh deterministic/real-process retry/reversal tests and AR08 blocked/resolved continuation in both directions. |
| Ordinary executables express realistic fixtures, generated-ID handoff, application edits and layered mutations. | Fresh generated-ID example and AR08 same-record layers; audited AR06/AR07 tool/service/edit loop. |
| Results identify actual roles/stages, accepted/pending state and appropriate next choices; failures stop without false external guarantees. | Fresh synchronized feedback, sparse printed-command and conditional-save-failure tests; AR08 raw role order/status. |
| Maintained guidance/examples agree with source; boundaries and important limits are honest; normal gates pass. | Final source/ADR comparison, local documentation audit, literal examples and the gates below. |

This standard does not require a feature count, every finding to be resolved, or every registry row to be exercised. All six requirements are satisfied within the stated environment and scope.

## Fresh AR08 — layered destination navigation

AR08 uses Python's standard library, application SQLite, JSON and loopback HTTP. Stages 1–3 reuse AR06's ordinary fixture/tool/service; a wrapper adds a controlled external acceptance condition to Stage 3's observational check. Two later stages add distinct contributions to the **same C association**, followed by a dependent record. This is disposable research source retained outside Git, not a new shipped sample or runner protocol.

| Accepted position | Logical application state |
| --- | --- |
| 0 | Only unrelated background user, association and activation remain. |
| 1 | Three run-owned users: no association, sparse association, complete C association with original status/contact. Generated IDs in JSON. |
| 2 | Python-tool HTTP reads captured; expected business deficiency is a successful tool result. |
| 3 | Tool/service changes C status `before` → `after`; independent SQLite check. |
| 4 | Ordinary Python executable changes C contact to `updated@example.invalid`, preserving Stage 3 status. |
| 5 | Dependent activation exists only when status and contact contributions are present. |

Fresh real processes walked **0 → 4 → 2 → 5 → 4 → 3 → 0**. Down 5 removed only activation, preserving contact/status; down 4 restored only contact, preserving status; down 3 restored status, preserving captured Stage-2 responses. IDs/user rows and earlier JSON bytes stayed stable. Cleanup removed only run-owned records/artifacts; the unrelated user, association and activation remained exactly unchanged. Baseline cleared the UUID; the next run received a different one.

The second run tested both farther continuations:

```text
up 5 → stages 1/2 accepted → 3/up succeeds → 3/verify-up blocked
     → accepted 2, pending 3/up; stages 4/5 untouched
up 5 while blocked → only 3/verify-up; still stopped
repair external condition
up 5 → 3/verify-up → 4/up/check → 5/up/check → accepted 5

down 0 → 5/down/check accepted → 4/down succeeds → 4/verify-down blocked
       → accepted 4, pending 4/down; Stage 3 contribution still present
down 0 while blocked → only 4/verify-down; no earlier cleanup
repair external condition
down 0 → 4/verify-down → 3/down/check → 2/down → 1/down/check
       → baseline, no UUID/pending, unrelated rows preserved
```

No successful pending mutation replayed. Exact expected role order, checkpoint rows, application rows, artifact hashes, role environment/cwd and status identities were asserted at each observation. The checks did not repair fixture state. The service was explicitly stopped. **28 CLI/database processes, 50 role attempts, two run UUIDs; EXPECTED on the first execution.** Four movement failures were intentional blocked verifiers. There was no new finding or harness correction, and no product code changed.

## Fresh verification and provenance

Actually exercised: **macOS 26.6.2 (25G83), arm64; Rust `1.94.0 (4a4ef493e 2026-03-02)`, Cargo `1.94.0 (85eff7c80 2026-01-15)`; Python 3.13.2, Python SQLite 3.49.1**. Scenario helpers used `PYTHONOPTIMIZE=1`; checks use explicit nonzero error handling. Unix-gated CLI tests executed. **The declared Rust 1.85 minimum, Linux Rust runner and native Windows were not exercised here.** Earlier Linux script-only receipts remain separate.

AR08 and literal examples exercised exactly **`265623137e3fdcac4de34b080439b340277fe93e`**. Gates exercised that unchanged implementation/example tree with the retained research records committed at `15ec7a4`. This finishing work changes only documentation. Exact source trees, documentation hashes, binary hashes, checkout state and final commit identity are retained with the external receipts; no finalization change alters Rust, dependencies, SQL or executable examples.

| Actual command / receipt | Fresh result |
| --- | --- |
| `cargo build --locked --workspace` | Exit 0; real binaries rebuilt and archived. |
| `python3 -B <evidence>/prior-audit.py` (`prior-evidence`) | Exit 0; read-only cross-check of inherited AR06/AR07, not scenario replay. |
| `python3 -B <evidence>/core-v0/scenario/run.py --repo <checkout> --evidence-root <new-output>` | Exit 0; AR08 outcomes above, per-command UTC/argv/cwd/environment/exit and raw streams. |
| `python3 -B <evidence>/readmes.py` (`literal-readmes`) | Exit 0; root quickstart/backout (existing-checkout clone/cd skipped only), every UUID-file walkthrough shell block, every generated-ID block including deliberate failure/repair/retry and read-only final row count. All ended at baseline. |
| `cargo fmt --check` (`final-fmt`) | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` (`final-clippy`) | Exit 0; no warnings. |
| `cargo test --locked --workspace` (`final-tests`) | Exit 0; **39 passed, 0 failed, 0 ignored**: Application 20, CLI 14, Database 4, Infrastructure 1. |
| `git diff --check` and `git diff --cached --check` (`final-diff`, `final-staged-diff`) | Exit 0; staged checking includes the new closeout record. |
| `python3 -B <evidence>/audit_docs.py` (`final-docs-audit-fixed-checker`) | Exit 0; local links/anchors, metadata/source targets, shell syntax, example permissions/shebangs, F01–F15/corrections and unchanged accepted decisions/production tree checked. Remote URL liveness not checked. |

The inherited documentation checker initially rejected the valid `core_v0_status` metadata key because its key pattern excluded digits. The checker was corrected to permit digits after the first character; the original source and failing `final-docs-audit` receipt remain under `harness-revisions/` and `checks/`. No document was changed to evade the check and no scenario replay was needed. This is a checker correction, not a product defect. A separate fresh read-only `acceptance-audit` also confirmed all four literal-example workspaces at baseline and AR08's counts, blocked-retry byte stability and scoped cleanup.

Raw evidence stays in the existing external observation area under **`2026-10-01_233459Z/`**. Portable anchors: `PLAN.md` (standard before execution), `prior-audit.py`, `prior-evidence-audit.json`, `core-v0/scenario/`, `core-v0/AR08/{metadata.json,result.json,commands.json,commands/,snapshots/,trace/,initial-workspace-source/,source.tar,binaries/,server-process.json}`, `literal-readmes/`, `checks/`, and final source/commit manifests. The reusable driver accepts checkout/output paths. Workspaces, databases, binaries, private absolute paths and raw transcripts are excluded from Git. Historical failed attempts/corrections remain untouched.

## Final authority check, limitations and next phase

Accepted ADRs, maintained design, findings and open questions were re-read against the final source. Guides/reference still own the executable contract; this record adds evidence and milestone standing. Examples match supported role/environment behavior. AR08's acceptance markers, service lifecycle, activation schema and research logs do not become public capabilities. ADR-0003 remains proposed; accepted ADRs and F01–F15 dispositions are unchanged.

Important retained limits: authors own compensation correctness, dependencies, generated-ID files and external effects. Nonzero/partially effective mutations and failed/ambiguous saves need author inspection; no automatic rollback or crash reconciliation. Status/settled targets are bookkeeping, not rechecks. Stage structure must stay stable during a run; concurrent instances are unsupported. Optional verifiers are rediscovered, including their removal while pending. Output is buffered per role, without durable history or live bytes. Schema verification is narrow version/history checking; malformed layouts can block status. Signals/process-tree cancellation and specialized launch diagnostics remain limited. See the [full findings and revisit triggers](2026-10-01-guided-usage-findings.md#findings-register--all-original-ids-retained).

No UI/server, scheduler/DAG, hosted execution, PostgreSQL/Docker/auth fidelity, generalized context/SDK, standalone reverify, readiness expansion or recovery system is required to call this core complete. There is no demonstrated conceptual blocker for its intended single-user loop in the exercised conditions.

The next phase is **repeated real owner tickets using this stable contract**. Let recurring authoring, setup, inspection, compensation or handoff friction determine small ergonomics changes. The owner's actual Python-tool/Node-service workflow should outweigh additional synthetic scenario volume. Revisit a deferred limit when real use encounters it, or rerun focused evidence after a relevant implementation change; do not reopen design solely because a registry row is untried.
