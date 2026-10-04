---
id: CT-GUIDED-USAGE-2026-10-01
title: Guided usage findings and core refinement evidence
type: research
status: recorded
created: '2026-10-01'
updated: '2026-10-04'
verified_on: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
method: archived-evidence-review-targeted-application-tests-real-process-sqlite-smokes-documentation-audit
sources:
- run-semantics-validation.md
- documentation-validation.md
- ../design/current-design.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- ../../crates/application/tests/run_semantics.rs
- ../../crates/cli/tests/workbench_cli.rs
- ../../examples/simple/workflows/generated-id/README.md
---

# Guided usage findings and core refinement evidence

The owner selected two phases from `CT-DEV-2026-10-01-CORE-USAGE` (`control-tower-core-usage-development-spec.md`): trustworthy movement feedback/checkpoint reporting, then a bounded authoring example and retained knowledge. The earlier broader development proposal is historical context. **Deferred entries below are known limitations or future candidates, not an implementation backlog or additional acceptance criteria for this build.** Revisit triggers are discussion inputs, not authorization to add features.

## Baseline and evidence provenance

The receiving checkout was clean on `test/workflows` at `c502fda5edb5d5bbfad1f194bff540ec22b459d0`, the same commit as the specification's reviewed `main`. Documentation, accepted ADRs 0001/0002/0004/0005, ADR-0003's proposed standing and implementation observation, Application, CLI, Infrastructure and Database contracts were read before implementation. No concrete contradiction requiring a semantics change or scope expansion was found. Accepted ADRs remain unchanged.

Inherited research archive: **`2026-10-01_174032Z.tar.gz`**, containing the root `2026-10-01_174032Z/`:

```text
SHA-256: 3c49aa9b7f3c803fbd6908734ea7f66a4d51f31f34b0575d3d0e44d5d091c114
Baseline: c502fda5edb5d5bbfad1f194bff540ec22b459d0
```

Read `REPORT.md` together with `CORRECTIONS.md`; corrected raw stdout/stderr and checkpoint snapshots take precedence over preliminary journal interpretations. The prior review compared archived trees, including executable modes, with the baseline:

| Tree | Recorded matching Git tree hash |
| --- | --- |
| `crates/` | `4109af7fc97a773144944e789369dc3d5eaa0ec4` |
| `docs/` | `23e2fbbd466135c6e311067bfbff7cd8b7d67765` |
| `examples/` | `88b7d330508735f0198187589f967cdb3256365d` |

The archive reports **44 experiments, 540 custom process launches, 31 existing tests passing, and 120 seeded navigation requests**. These counts are inherited execution receipts, not freshly repeated in this implementation. The seeded model varied navigation targets/verifier failures; it does not establish arbitrary mutation/save/interruption/concurrency correctness. Disposable DB and loopback HTTP fixtures prove an authoring boundary, not production-ticket suitability or usage frequency. E39's successful 2.1 MB capture is not a memory-safety bound.

New raw evidence is durable **outside Git**, under the owner's observation area in `implementation/2026-10-01_194310Z/`. It contains `SPEC.md`, `JOURNAL.md`, the executable `record.py`/`smoke.py` harnesses, command JSON receipts (argv, cwd, UTC, exit), separate unmodified stdout/stderr, transcripts, fresh fixture copies and database snapshots. This document is the public synthesis; raw archives, binaries, private absolute paths and generated workspaces are not committed.

## Implemented changes and preserved boundaries

Phase 1 commit **`03478f5f5a4a6ee9a9cb733d81e7f909bf0361d0`** adds Application-owned synchronous start/finish observations, keeps captured bytes in results, and lets CLI render each role before the next invocation. The final summary uses actual stage numbers/labels and the same bookkeeping representation as status. Failed-verifier choices come from Application, resolve only the active stage, and are shell-quoted using the current executable/absolute workspace. Mutation/save failures and failed reverse mutations do not acquire verifier recovery recipes from old pending state.

The same commit saves a proposed checkpoint before publishing it as confirmed state. Failures retain the latest confirmed checkpoint, attempted update, original storage cause, and all executed role results; no later role/write follows. It covers UUID allocation, pending publication, acceptance and baseline clearing without repair/retry machinery. CLI clearly separates process success from checkpoint confirmation.

Phase 2 example commit **`3ec3039f99ba857df9df98535412e23213baa8ac`** adds eight thin executable roles and one standard-library Python helper in [the generated-ID example](../../examples/simple/workflows/generated-id/README.md). Two stages carry a SQLite-generated integer ID in author-owned JSON through create/change/restore/delete. Checks use read-only application DB connections and fail rather than create/repair data. Python is an optional example prerequisite, not a Control Tower runtime requirement or a prerequisite for the original quickstart.

Navigation, optional-role policy, settled-target no-op, UUID lifetime, persistence schema, five environment variables, direct process launching, null stdin, and explicit DB setup remain unchanged. Production Infrastructure and Database are unchanged. The CLI's only new dependency is **test-only** `rusqlite` for real conditional faults; no dependency versions changed. No new command, driver, managed context, event bus, async runtime, output store, recovery/history mechanism, schema readiness expansion, UI/server, locking or drift protection was added.

## Newly executed acceptance

Environment actually exercised: **macOS 26.6.2 (25G83), arm64**, `rustc 1.94.0 (4a4ef493e 2026-03-02)`, Cargo `1.94.0 (85eff7c80 2026-01-15)`, Python **3.13.2**. Unix-gated real-process tests ran here. Other platforms and the declared minimum Rust version were not exercised.

Phase 1 acceptance was recorded complete **before any Phase 2 implementation**. Exact commands from the checkout root and their recorded outcomes:

| Command / receipt | Executed result |
| --- | --- |
| `cargo build --locked --workspace` (`phase1-build-final`) | Exit 0; both real executables built. |
| `cargo fmt --check` (`phase1-fmt-final`) | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` (`phase1-clippy-final`) | Exit 0; no warnings. |
| `cargo test --locked --workspace` (`phase1-tests-final`) | Exit 0; **39 passed, 0 failed, 0 ignored**. Application 20, CLI 14, Database 4, Infrastructure 1. |
| `git diff --check` (`phase1-diff-final`) | Exit 0. |
| `python3 <evidence>/smoke.py phase1-smoke` (`phase1-manual`) | Exit 0; **31 actual CLI/DB processes** with per-command receipts. |
| `python3 <evidence>/smoke.py generated-id-smoke` (`phase2-generated-fixed-harness`) | Exit 0; **29 actual processes**, row/checkpoint snapshots and exact author-owned role-call log. |
| `python3 <evidence>/smoke.py literal-readmes` (`phase2-readmes`) | Exit 0; root quickstart/backout and every new example shell block executed through `sh -ex`, with expanded command traces retained. |
| `python3 <evidence>/check_example.py` (`phase2-example-syntax`) | Exit 0; Python syntax, all eight executable/shebang-led roles, shell syntax, no generated DB/bytecode in source. |

The initial locked test invocation failed because adding the test-only SQLite dependency required refreshing the lockfile. `cargo check --offline --workspace --all-targets` refreshed it; all subsequent locked runs passed. The first Phase 2 harness invocation had a Python syntax error in authored logging, before any example ran; its receipt is retained. Correcting the external harness produced the passing result above. Neither failed attempt is relabeled as successful acceptance.

The first documentation-checker run rejected nested metadata in an unchanged historical record; the flat metadata checker was corrected to inspect top-level fields there. It then passed without changing historical documents. This was a checker error, not a project defect. Remote URL liveness was not fetched; local targets/anchors and updated metadata source paths were checked.

### Gate-to-evidence mapping

| Gate | Actual evidence and observation |
| --- | --- |
| A1 — navigation | All existing assertions retained: normal walk, optional verifiers, sparse targets, no-op/invalid/unreachable targets, both retry/reversal directions, reverse-check failure, UUID continuity/clearing. Fresh `phase1-smoke` walked up 3 → down 1 → up 3 → down 0, checking file contents and the same UUID. |
| A2 — timely feedback | CLI `role_feedback_arrives_while_roles_wait_and_output_is_not_replayed` observes a start while the real child waits and the first output/finish while the second waits. Application `synchronous_observations_surround_each_role_before_the_next_invocation` asserts start/run/finish ordering before the next invocation and exact raw bytes. Captured no-newline output appears once; stream framing is separately tested. |
| A3 — results/choices | Application `verifier_choices_resolve_only_active_sparse_stage_and_exclude_other_failures`; CLI `sparse_retry_and_reversal_commands_are_usable_and_resolve_only_the_active_stage` executes printed commands from `/` with an apostrophe/space workspace, uses stages 10/200/900 and verifies only the active stage is resolved. A same-pointer pending reversal executes roles. Missing reverse and failed-reverse-mutation cases have no misleading recipe. Fresh smoke repairs stage 3's check, observes verifier-only retry, then exercises pending reversal. |
| A4 — checkpoint truth | Application `failed_checkpoint_publication_returns_latest_confirmed_state_and_attempted_update` injects nine save cases: initial UUID, pending, final acceptance, no verifier, baseline clearing, earlier progress, reverse pending, reverse baseline and verifier-only acceptance. It asserts confirmed/attempted state, source cause, retained bytes, role/write order and no subsequent work. |
| A5 — real SQLite | CLI `conditional_sqlite_save_failures_report_confirmed_checkpoint_and_retain_role_output` and fresh `phase1-smoke` use conditional fail-before-write triggers. Pending-save fault retains stage 1/up output, UUID/no pending, no verifier. Final-save fault retains up and verify-up output, baseline/pending up 1. Exact confirmed display equals subsequent status. After trigger removal, pending-save retry runs mutation/check; final-save retry runs only check. |
| A6 — example | Fresh actual processes walk 0 → 1 → 2 → 1 → 2 → 0, query external row values, compare generated ID/UUID, remove row/handoff and assert recorded baseline. Failed stage 2 verifier is repaired and retried; role log grows by exactly `2 verify-up`. All four verifier entry points fail against missing application DB and missing schema without changing files/schema, under `PYTHONOPTIMIZE=1`; a wrong-value check fails without repair. Root README's clone/cd lines alone are skipped as documented for an existing checkout; remaining literal quickstart/backout runs. Python smoke is separate and was executed, not skipped. |
| A7 — documentation/scope | Updated canonical CLI/executable references, navigation/authoring/troubleshooting, maintained design/index and optional root example link. All F01–F15 and corrections are retained below. Documentation link/metadata/shell/mode audit and literal guide runs are recorded in the final receipt table. Accepted decisions and original quickstart commands remain intact. |

### Final gates and guide audit

Executed against `3ec3039f99ba857df9df98535412e23213baa8ac` plus this round's documentation changes; the core code is the validated `03478f5` implementation. The final documentation commit records this synthesis without altering the tested implementation/example.

| Actual command / receipt | Outcome |
| --- | --- |
| `cargo fmt --check` (`final-fmt`) | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` (`final-clippy`) | Exit 0; no warnings. |
| `cargo test --locked --workspace` (`final-tests`) | Exit 0; **39 passed, 0 failed, 0 ignored**; Unix CLI integrations executed. |
| `git diff --check` (`final-diff`) | Exit 0, including the new findings record. |
| `python3 <evidence>/smoke.py literal-guides` (`phase2-guides`) | Exit 0; literal marker-authoring guide and navigation guide ran, including **both** pending-failure retry/backout alternatives. Expanded shell commands/output and fresh workspaces retained. |
| `python3 <evidence>/audit_docs.py` (`phase2-docs-audit-fixed-checker`) | Exit 0; 27 Markdown files, 24 unique metadata IDs, 250 link references inspected, local targets/anchors checked, 54 updated metadata source targets, 28 changed shell blocks checked. Role modes/shebangs/syntax, all F IDs/corrections, unchanged root quickstart, accepted ADRs and production adapters checked. |

These are fresh executed checks. The new example's 29-process Python smoke and literal README smoke were separate, explicit acceptance, not ignored Rust tests. They require the documented optional Python runtime. Captured output remains buffered per role; persistence is not an external transaction. Cancellation, drift/concurrency, comprehensive readiness, degraded status, live streaming, standalone check and durable product history remain deferred, with no new cross-platform or minimum-toolchain claim.

## Findings register — all original IDs retained

“Resolved” below refers only to the selected observable behavior and its named implementation/evidence. It does not turn related deferred capabilities into acceptance criteria. E paths are relative to the inherited archive root; their original failures remain historical evidence.

| ID | Corrected observation / inherited evidence | Disposition and fresh evidence | Retained consequence / revisit trigger |
| --- | --- | --- | --- |
| **F01 — checkpoint result truth** | E41; E32 corrected. Save failure exposed attempted rather than confirmed position; child output was retained. | **Selected defect resolved in `03478f5`**: A4/A5 tests and fresh conditional-fault smoke confirm latest saved state plus attempted update/output/cause. | External effects are not reconciled; a failed storage call can be ambiguous. This correction does not promise recovery. |
| **F02 — schema verification blind spot** | E22: version/history can pass a missing/malformed checkpoint table. The documented operation was always narrow. | **Documented; readiness/repair changes deferred.** CLI reference and troubleshooting explain limits. No new readiness gate. | Revisit actual setup/upgrade/workspace failures. Retain malformed legacy adoption and upsert unique-key assumptions as future cases. |
| **F03 — stage identity presentation** | E04/E30: stage 200 was described as index 2 despite correct traversal. | **Selected defect resolved in `03478f5`**: A3 real sparse-stage command execution, summaries/status use actual IDs/labels. | Counts remain explicitly labeled; internal persistence indexes are unchanged. |
| **F04 — malformed-layout status** | E19/E43: role-as-directory blocks status; chmod alone does not. E38's preliminary hypothesis was refuted. | **Documented; degraded-status architecture deferred.** No invalid-layout stage mapping inferred. | Revisit repeated ordinary authoring failures that hide useful IDs/progress. Status also composes a write adapter; practical read-only-access consequences remain a source-inspected hypothesis. |
| **F05 — whole-movement output delay** | E25/E29: first output waited for the whole route. E39 captured 2.1 MB, without establishing an output bound. | **Role-level behavior resolved in `03478f5`**, A2 synchronized real-process/event-order tests. **Live byte streaming/output limits deferred.** | Revisit a real long-running role needing intermediate logs. Buffering lasts for one role; logs are not persisted. |
| **F06 — no settled-state check** | E05/E24: external drift leaves saved status unchanged; same settled target is a no-op, by intent. | **Documented; standalone check deferred.** Status/no-op remain metadata-only; no command added. | Revisit repeated accepted-fixture rechecks without reverse/reapply; define observation semantics separately. |
| **F07 — partial mutation cleanup** | E12/E13/E42: failed first up can leave effects, baseline, retained UUID and no pending check. Down 0 is a no-op; retry can repeat effects with the same UUID. | **Documented; conservative failure messaging retained; recovery command deferred.** Mutation/save failures do not get verifier recipes. | Revisit repeated real partial-effect cleanup pain. Authored/manual cleanup works only if it handles the partial state; deleting DB is never rollback. |
| **F08 — application-generated IDs** | E17/E36: author-owned ID files worked with local SQLite and a loopback API. E17's helper initialized schema during checks. | **Selected small example resolved in `3ec3039`**: A6 fresh row/value/ID/call tests and literal example README. **Managed handoff/HTTP sample deferred.** | JSON publication and row/runner writes remain separate. Revisit recurring real context-file friction; no stdout parser or SDK. |
| **F09 — later missing mutation** | E14/E15: earlier transitions can be accepted before a later missing role stops traversal. | **Documented; incremental traversal preserved.** Existing missing-role semantics assertions remain green. | Revisit repeated capability-visibility confusion; do not infer atomic routes or all-route preflight. |
| **F10 — no execution history** | E31: SQLite holds the latest checkpoint, not attempts/output/old UUIDs. | **Documented; persistent journal/log viewer deferred.** Raw research evidence is external and does not become product history. | Revisit debugging information repeatedly unavailable from captured terminal output or authored logs. Migration history is not run history. |
| **F11 — interruption delivery** | E26: parent-only SIGTERM allowed a later child write; group SIGINT stopped it in that experiment. Both retained UUID. | **Documented; process-tree/cancellation/crash recovery deferred.** No new interruption experiments claimed here. | Revisit the actual launcher/terminal behavior encountered in work; this does not prove all Ctrl-C use broken. |
| **F12 — no-shebang success on host** | E18/E44: executable shell text without shebang ran on tested macOS; fallback mechanism untraced. Initial E18 failure was the verifier. | **Corrected observation documented; shebang guidance retained.** No fallback/prohibition added. | Revisit explicitly supported platforms/strict validation. This host receipt is not portability evidence. |
| **F13 — structural drift** | E23: inserted stages reinterpret persisted counts and can leave old effects after down 0. | **Documented; identity/drift machinery deferred.** Finish/clean run before structural edits. | Revisit unavoidable structural edits during active real work; restart does not reconcile counts or effects. |
| **F14 — removing pending check** | E40: deletion of optional verifier allows acceptance without replay/check. | **Documented; optional-role policy preserved.** No frozen-run verification. No-role wording does not claim every such movement started settled. | Revisit only if the owner selects stronger frozen-run semantics. This is not a security-boundary bypass. |
| **F15 — launch/signal diagnostics** | E18/E33: missing interpreter/CRLF can report ENOENT despite existing role; signal result omits specific identity. | **Original cause and actual role identity retained in `03478f5`; specialized diagnosis deferred.** Current guidance explains author checks. | Revisit repeated real launch failures needing more context; do not install runtimes or guess OS causes. |

## Corrections preserved explicitly

- **E12/E42:** UUID is allocated before mutation, retained after failed attempts/down-0 no-op, and reused on retry. The early contrary sentence is wrong.
- **E18/E43/E44:** execute-bit removal does not block status. Shebangless success occurred on this host; early shebangless failure was verification failure, not failure to launch up.
- **E22:** the schema blind spot concerns the narrow version/history contract, not an invented comprehensive health promise.
- **E26:** both interrupted runs retained the UUID; keep parent-only and process-group delivery distinct.
- **E32/E41:** the unconditional initial write fault prevented all roles; conditional later faults retained child output and exposed attempted/confirmed position disagreement.
- **E38/E43:** role-as-directory reproduced unavailable status, not chmod alone.
- **E20 command 004:** two before/after snapshots used the wrong cwd base. They cannot prove an empty workspace; command results and later absolute-path observations are supporting evidence.

The prior review's missing-upsert-key probe was **SQL-only through Python SQLite**, using the committed SELECT/upsert. A readable table without a suitable primary/unique key can fail `ON CONFLICT(id)` preparation. It was not a Rust integration result and is a future F02 candidate, not a new gate. Status's composition of both query/write adapters was inspected in source; no practical permission failure is claimed without reproduction.

E17's published-sample correction is author-owned: read-only checks now avoid the original helper's unconditional schema initialization. The runner cannot enforce observational verifiers. Three-command setup, installed-command discovery, default workspace choice, aliases/completions and machine-readable status remain possible ergonomics discussions; E01/E21 do not authorize implementing them. The next product input is an actual work ticket exercised through the improved loop.

## Archive anchors and reproduction receipts

Original provenance: `metadata.json`, `documentation-manifest.json`, `source-snapshot/`, `REPORT.md`, `CORRECTIONS.md`. Selected archive anchors:

```text
experiments/01-quickstart/transcript.md
experiments/02-iterative-navigation/transcript.md
experiments/06-retry-up-verifier/transcript.md
experiments/07-backout-pending-up/transcript.md
experiments/10-failed-reverse-verifier/transcript.md
experiments/11-baseline-pending-uuid/transcript.md
experiments/17-application-db-handoff/transcript.md
experiments/17-application-db-handoff/workspace/support/app.py
experiments/25-buffered-output-timing/timing.json
experiments/30-sparse-pending-diagnostics/transcript.md
experiments/35-randomized-navigation/model-trace.json
experiments/41-conditional-checkpoint-failures/transcript.md
experiments/41-conditional-checkpoint-failures/commands/
experiments/41-conditional-checkpoint-failures/snapshots/
experiments/42-failed-mutation-uuid-and-cleanup/transcript.md
```

`REPORT.md` maps all deferred E items to their directories. Fresh receipts are separately under `implementation/2026-10-01_194310Z/`: `phase1-tests-final.*`, `phase1-manual.*`, `phase1-smoke/`, `phase2-generated-fixed-harness.*`, `generated-id-smoke/`, `phase2-readmes.*`, `literal-readmes/` and final-gate/audit receipts. These are new executions, not replacements for the inherited archive.

## October 2, 2026 example relocation

Live example links now target the family `workflows/` layout (the former `workspaces/` directory, renamed on 2026-10-04 when the Workspace → Workflow → Stage terminology was adopted; see [ADR-0007](../decisions/0007-desktop-ui-shell.md)). Earlier command paths and validation revisions above retain their original historical meaning.
