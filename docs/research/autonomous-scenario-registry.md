---
id: CT-AUTONOMOUS-SCENARIOS
title: Autonomous usage scenario registry
type: research
status: recorded
created: '2026-10-01'
updated: '2026-10-01'
verified_on: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
method: corrected-prior-evidence-review-and-disposable-real-cli-scenarios
sources:
- 2026-10-01-guided-usage-findings.md
- autonomous-usage-runs.md
- ../reference/cli.md
- ../reference/stage-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- 2026-10-01-core-v0-completion.md
---

# Autonomous usage scenario registry

This registry selects usage research, not implementation work. Classification and revisit triggers confer no development authority. Read maintained guidance, accepted ADRs, the corrected [findings register](2026-10-01-guided-usage-findings.md), this registry and [run summaries](autonomous-usage-runs.md) before each cycle. Record the exact revision and select a small set with an explicit information gain. Repetition requires an important new dimension or a relevant implementation change.

Statuses: `untried`, `covered`, `needs-variation`, `revisit-on-change`. `covered` applies only to listed dimensions; it is not universal certification. Revisions below abbreviate full hashes recorded in the run summaries/findings register.

## Inherited coverage

These families index all 44 experiments in external archive `2026-10-01_174032Z/`, exercised at **c502fda**. They were reviewed, not rerun in this cycle. Initial interpretations are superseded by that archive's CORRECTIONS.md and the maintained findings register. Refinement tests/smokes at **03478f5** and **3ec3039**, documented in **2656231**, add separately recorded evidence for feedback, sparse identity, checkpoint reporting and generated-ID authoring.

| ID | Intent / category | Status | Revisions exercised | Related findings | Dimensions already exercised | Useful next variation / trigger |
| --- | --- | --- | --- | --- | --- | --- |
| H01 | Ordinary shell navigation / successful workflows | covered | c502fda; 03478f5 (refinement acceptance) | F03, F06, F10 | E01–E05, E29–E30: quickstart, separate CLI processes, optional checks, sparse IDs, settled/wrong/unknown targets, 75-stage walk. | Real application stages with ordinary edit/backout/reapply; AR01 supplies a compiled variation. |
| H02 | Pending verification lifecycle / recoverable failures | covered | c502fda; 03478f5 | F03, F14 | E06–E11, E35, E40: same-direction check retry, both reverse directions, failed reverse checks, baseline UUID, 120 seeded requests, deleting pending optional check. | Longer mixed application route where resolving a check then walks to a farther target. |
| H03 | Runtime and path contract / authoring | covered | c502fda; 03478f5 (printed-command acceptance) | F12, F15 | E16, E20, E27–E28, E34, E44: Python env/cwd, Unicode/spaces/symlink/relative paths, null stdin, binary output, symlink roles/stages, host shebangless behavior. E20's two wrong-cwd snapshots are nonauthoritative. | Direct compiled roles (AR01); author-owned Python venv/project CLI (AR04). |
| H04 | Generated-ID fixture handoff / application integration | needs-variation | c502fda; 3ec3039 | F08 | E17, E36: separate SQLite and Node/loopback HTTP fixtures, JSON IDs, business error/retry, cleanup. E17 check helper originally initialized schema; published example checks are read-only. | Mixed DB/API workflow (AR02); owner's real workflow when available (AR06). |
| H05 | Script/layout mistakes / recoverable authoring | revisit-on-change | c502fda; 03478f5 | F04, F09, F12, F15 | E14–E15, E18–E19, E33, E38, E43–E44: missing later mutation, modes, role directories, numeric/filename mistakes, missing interpreter/CRLF, child signal. Chmod alone permits status. | Recurring ordinary project-runtime error; avoid repeating known malformed-layout/signal probes without new evidence. |
| H06 | Setup and schema / persistence boundaries | revisit-on-change | c502fda | F02 | E21–E22, E37: setup reruns, absent/uninitialized/corrupt storage, narrow version/history check, legacy pending adoption. Missing upsert key remains SQL-only prior evidence. | Actual setup/upgrade failure or changed readiness contract. |
| H07 | Failed mutation effects / deferred recovery | revisit-on-change | c502fda | F07 | E12–E13, E42: partially effective up/down, baseline down no-op, retained/reused UUID, repeated effects, manual authored cleanup. | Concrete application partial-failure pain; no recovery guarantees inferred. |
| H08 | Long roles and output / ergonomic pressure | needs-variation | c502fda; 03478f5 | F05 | E25, E29, E39: slow roles, 75 stages, 2.1 MB capture; original whole-walk delay and framing refined with synchronized role-level evidence. Live bytes remain buffered per role. | Longer mixed-runtime iteration (AR05) or a real role whose intermediate logs matter. |
| H09 | Checkpoint truth / storage failures | revisit-on-change | c502fda; 03478f5 | F01 | E32, E41: initial unconditional fault prevented roles; conditional faults retained output and exposed old reporting defect. Refinement tests/smokes distinguish confirmed and attempted state and retry effects. | Implementation change or actual ambiguous checkpoint failure; no automatic external reconciliation. |
| H10 | Stored state and external drift / persistence | needs-variation | c502fda | F06, F10, F13 | E23–E24, E31: unsupported stage insertion, settled external drift, latest checkpoint only. | Quiescent copy/reopen with unchanged stage structure (AR03); actual unavoidable structural edits only if new information. |
| H11 | Interruption delivery / deferred process edge | revisit-on-change | c502fda | F11 | E26: parent-only SIGTERM permitted a late child write; process-group SIGINT stopped it on this host; both retained UUID. | Actual launcher/terminal cancellation behavior or a changed process contract. |

## Autonomous scenarios

Raw evidence is outside Git in the owner's existing observation area. Cycle identifiers and relative evidence anchors are portable; private absolute paths, binaries, workspaces and raw transcripts stay external.

| ID | Intent / category | Status | Revisions exercised | Related findings | Dimensions exercised / evidence | Useful next variation |
| --- | --- | --- | --- | --- | --- | --- |
| AR01 | Edit/backout/rerun shell, Python and direct C roles / successful authoring | covered | 265623137e3fdcac4de34b080439b340277fe93e | F03, F05, F06, F10 | Cycle 001 `AR01/`: sparse 10/200/900, optional checks, direct native roles with no argv, stage cwd, unrelated command cwd, apostrophe/space workspace, no-newline native streams, two run UUIDs, changed input. 18 CLI/DB processes, one compiler process, 32 role attempts. EXPECTED. | Real project CLI build-and-invoke loop with an ordinary dependency/author error. |
| AR02 | Coordinate separate DB/API IDs through repair/reversal/application edit / integration | covered | 265623137e3fdcac4de34b080439b340277fe93e | F08, F03 | Cycle 001 `AR02/attempt-02/`: Python SQLite helper, native Node roles, shell composition, sparse stages; intentional API read bug; verifier-only retry; same IDs through changed-v2 and restoration; full cleanup; 13 CLI/DB processes, 17 roles, 13 HTTP requests. EXPECTED. Initial driver assertion failure preserved at `AR02/`. | Continue farther after pending resolution in a mixed application route; owner workflow takes priority. |
| AR03 | Reopen copied pending workspace from another cwd / persistence | covered | 265623137e3fdcac4de34b080439b340277fe93e | F08, F10 | Cycle 001 `AR03/attempt-02/`: quiescent full copy with runner/application SQLite, handoff and executable modes; relative and absolute invocations; copied check repair; unchanged original; independent cleanup. 14 CLI/DB processes, 14 fresh roles (inherited copied log lines excluded). EXPECTED. Initial driver assertion failure preserved at `AR03/`. | Workspace move/rename with author-owned relative context, or explicit external absolute-path dependency to distinguish fixture portability from bookkeeping. |
| AR04 | Invoke a local project CLI from an author-owned Python venv / ordinary author mistakes | untried | None | F15; H03/H05 | No executed evidence. | Verify interpreter/package failure before effects, then fix only disposable author setup and rerun. |
| AR05 | Repeated inspection through a longer mixed-runtime route / ergonomic pressure | untried | None | F05, F10; H08 | No executed evidence; inherited 75-stage shell walk is separate. | Use bounded meaningful application states and inspect usefulness of role results during several edits. |
| AR06 | Owner-described Python tool→HTTP/key→persisted user associations / developer usage | covered | 265623137e3fdcac4de34b080439b340277fe93e | F08, F06, F10; H02/H04 | Cycle 002 `AR06/execution/`: Python standard library only, SQLite, loopback GET/PATCH with synthetic key, three UUID-owned absent/sparse/complete users, unrelated background, JSON IDs/results, direct read-only persistence checks, reversal/repeat/scoped cleanup. 24 CLI/DB processes, 38 roles, 24 tool/HTTP calls; two run UUIDs. Acknowledge-without-save variation fails verification, retries only check, reverses and reapplies. EXPECTED. This models the supplied workflow shape, not a production environment. | Repeat with an ordinary author edit or continue beyond a repaired pending check; retain the same minimal infrastructure. Actual production-framework fidelity is outside this scenario. |
| AR07 | Edit application implementation during pending verification / developer iteration | covered | 265623137e3fdcac4de34b080439b340277fe93e | F08, F06, F10; H02/H04 | Cycle 003 `AR07/execution/`: AR06 roles/verifiers unchanged; ordinary application code commits incorrect while HTTP/tool succeeds. Printed verifier-only retry fails without replay. One source assignment is edited while pending; SQLite/checkpoint/artifacts remain unchanged. Printed down2 restores before; up3 executes corrected source and independently verifies after. Executed source hashes distinguish each mutation; captured Stage-2 results remain byte-identical; down0 preserves unrelated rows and clears run state/artifacts. 18 CLI/DB processes, 16 roles, 9 tool/HTTP calls, 4 application writes. EXPECTED; no special recovery or new finding. | AR04's ordinary project-runtime authoring error; an application with explicit author-owned restart/reload could separately test that lifecycle cost. No Control Tower reload behavior is inferred. |
| AR08 | Meaningful destinations, same-record layers and farther pending continuation / core completion | covered | 265623137e3fdcac4de34b080439b340277fe93e | F08, F06, F10; H01/H02/H04 | Cycle 004 `core-v0/AR08/`: five-stage AR06 variation; status/contact mutations on the same association, dependent activation; 0→4→2→5→4→3→0 preserves earlier contributions/results/IDs. Pending up3→5 and down4→0 retry only blocked checks, stop later stages, then continue after repair. Two UUIDs; scoped cleanup preserves unrelated user/association/activation. 28 CLI/DB processes, 50 roles. EXPECTED on first execution. | Repeated actual owner tickets, or a relevant implementation change; no additional synthetic variation required for core v0. |

Cycle 001 adds evidence to existing F03/F05/F06/F08/F10; it creates no new finding IDs and changes no disposition or accepted decision. Two initial driver assertions confused zero-based pending indexes with one-based positions; the corrected attempts are separate from those INCONCLUSIVE complete-scenario attempts.

Cycle 002 covers AR06 using the owner's explicit minimal scenario contracts. It adds F08 handoff/cleanup evidence and confirms H02 pending-check semantics with independent persistence observations. No new finding ID, product contradiction or changed disposition was established. AR04/AR05 remain untried. Source bundle and raw evidence remain outside Git.

Cycle 003 covers AR07's actual application implementation edit while verification is pending. Normal opposite-direction compensation and reapplication exercise the changed source without recreating earlier stages. The disposable service compiles current application bytes per PATCH and logs their hash; that is author-owned fixture behavior. Source edits and verifier retries do not alter existing persisted state. No new finding ID/disposition, product behavior or accepted decision changes. AR04/AR05 remain untried.

Cycle 004 closes the bounded [core-v0 standard](2026-10-01-core-v0-completion.md). AR08 covers the longer meaningful route and pending-continuation variation together; a fresh read-only audit supports inherited AR06/AR07 claims without relabeling them new executions. AR04/AR05 remain untried in their originally listed runtime dimensions and are not completion requirements. Earlier next-cycle recommendations remain historical suggestions. Repeated real owner use now takes priority; no new findings, implementation change or accepted decision resulted.
