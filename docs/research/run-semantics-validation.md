---
id: CT-RUN-SEMANTICS-VALIDATION
title: Run semantics decision alignment and validation
type: research
status: recorded
created: '2026-10-01'
updated: '2026-10-01'
verified_on: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
method: decision-reading-application-tests-real-sqlite-cli-processes
sources:
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
- ../decisions/0004-session-storage-port-and-adapters.md
- ../decisions/0005-cli-first-driving-adapter.md
- discovery-01.md
- playbook-compliance.md
---

# Run semantics decision alignment and validation

Inspected first: clean `main` at `1d1c83cd2a29701d798864284e7a31b3951e5ad3`. Read the documentation index, ADRs 0002–0005, current design, discovery brief/record, history, earlier probe and compliance ledger before changing implementation. Accepted decisions govern; historical checkpoint/patch and memory-only proposals do not.

## Decision-to-code trace recorded before implementation edits

All Application symbols below are in `crates/application/src/lib.rs`. Existing CLI evidence is in `crates/cli/tests/workbench_cli.rs`.

| Authority | Required observable behavior | Existing responsible code | Existing evidence | Mismatch at inspected HEAD |
| --- | --- | --- | --- | --- |
| ADR-0002 directional completion | Up commits higher position only after optional verify-up. | `move_to`, `finish_transition` | CLI forward walk and failed-up-verifier retry | No known normal-path mismatch; deterministic checkpoint timing coverage needed. |
| ADR-0002 directional completion | Down commits lower position only after optional verify-down. | `move_to`, `finish_transition` | CLI backward walk and failed-down-verifier retry | No known normal-path mismatch; deterministic checkpoint timing coverage needed. |
| ADR-0002 optional roles | Absent verifier completes on mutation success; absent mutation stops. | `move_to`, `Stage::executable` | CLI omitted-verifier test | Missing mutation/reverse optional-verifier coverage needed. |
| ADR-0002 active transition | Failed verification keeps the last completed position distinct from successful mutation. | `move_to`, `load_state` | Both CLI retry tests | `load_state` permits only normal pending states, rejecting reverse-verification states. |
| ADR-0002 retry | Matching verifier alone runs on continuation, including after reverse verifier failure. | Pending branch of `move_to` | Both normal CLI retry tests | No reverse-verifier retry coverage; branch prohibits initiating reverse. |
| ADR-0002 reverse action; discovery acceptance; history E15–E18 | Opposite mutation for the same active stage, followed by optional matching verifier; both directions supported. | Pending branch of `move_to`, `load_state`, `finish_transition` | None | Opposite direction rejected as conflicting; invariant assumes completed side from pending direction. |
| ADR-0002 migration order; discovery target example | Resolve active stage first, then walk each intermediate stage to requested target; impossible direction stops; settled target no-op. | `move_to`, `target_count` | CLI multi-stage forward/backward walk | Pending reversal cannot resolve or continue farther. |
| ADR-0003 minimal handoff; discovery fixture | Same opaque UUID throughout scripts/checks; clear after verified baseline with no active transition. | `move_to`, `run_role`, `finish_transition` | CLI full UUID fixture walk | No new value needed; explicit every-role UUID and baseline rollback-failure coverage needed. |
| ADR-0002 failed mutation | Report and stop, no verifier/later stage/completion or automatic cleanup. | `move_to`, `run_role` | CLI failed mutation test | Preserve narrow behavior; test both directions and failed reverse without recovery invention. |
| ADR-0002 session scope; ADR-0004 storage | Pending state and UUID survive ordinary separate CLI processes. | `load_state`, Query/Write ports; Database `workbench` adapters | CLI retry tests, real SQLite round-trip/adoption | Existing columns suffice, but reverse pending states were rejected by Application. |
| ADR-0004/0005 ownership | Application decides transitions; SQLite maps/stores; CLI translates intent and composes. | `Workbench`; Database `workbench`; CLI `commands`/`deps` | Compliance ledger, Cargo graph, setup test | No architectural contradiction exposed; preserve package/lane/error/setup boundaries. |

ADR-0002's generic directional active-transition rule supports the symmetric reverse of pending down. Discovery gives the concrete pending-up example and does not contradict symmetry. For stage index `i`, last completed count may be `i` or `i + 1`, independently of the most recently successful mutation direction. Direction selects the destination (`up: i + 1`, `down: i`); verifier acceptance alone commits it. No additional persisted field is needed.

## Implemented correction

`Workbench::move_to` selects the requested side of the active stage before comparing the settled target. Continuation runs only verification; reversal runs the opposite mutation. Both paths use `Workbench::run_transition`, which records successful mutation as pending before verification, commits only after acceptance, and stops on any failure. `load_state` accepts either adjacent completed count independently of pending direction. `finish_transition` already selects the correct directional destination and clears the UUID only on settled baseline; no change was needed there.

The persisted fields, schema, production SQLite adapters, package manifests, ports, source-preserving errors, CLI command handlers, explicit composition and database operational tooling are unchanged. The obsolete conflicting-direction failure is removed; an unreachable target reports `DirectionDoesNotReachTarget` without executing anything. Missing required mutation stops without fallback, including during reversal. A nonzero reverse mutation preserves the previous checkpoint and stops; it does not infer cleanup or external-state recovery.

## Final decision-to-code alignment

Application paths: `crates/application/src/lib.rs` and deterministic tests `crates/application/tests/run_semantics.rs`. Real CLI/process tests: `crates/cli/tests/workbench_cli.rs`. Real storage tests: `crates/database/tests/sqlite_boundaries.rs`. Test identifiers below name executed tests, not proposed exercises.

| Document / decision | Required behavior | Implementation symbol / path | Test evidence | Result |
| --- | --- | --- | --- | --- |
| ADR-0002 verified directional completion | Each up/down commits its own destination only after mutation and supplied verifier success. | Application `run_transition`, `finish_transition` | Application `verified_walks_commit_each_stage_and_share_one_uuid`; CLI `walks_fixture_forward_and_backward_across_cli_processes` | Aligned. Observed per-role checkpoints retain the previous completed count through verification. |
| ADR-0002 optional roles | No verifier: mutation success completes; absent required mutation: traversal stops. | Application `run_transition`, `Stage::executable` | Application `absent_verifiers_complete_both_directions`, `reverse_without_verifier_completes_immediately_in_both_directions`, `missing_mutations_stop_normal_and_reverse_walks_without_fallback`; CLI `omitted_directional_verifiers_do_not_block_movement` | Aligned in both directions, including reversal. |
| ADR-0002 active transition; history E15–E18 | Completed position stays distinct from the most recently successful unverified mutation; both adjacent completed sides are valid. | Application `WorkbenchState`, `PendingTransition`, `load_state` | Application `validates_both_adjacent_completed_positions_independently_of_direction`, `failed_reverse_verifier_retries_without_mutation_replay_on_either_side`; real SQLite round-trip covers all four combinations | Aligned without a new field or schema change. |
| ADR-0002 verifier retry | Retry matching verifier only; retain completed position on failure. | Application pending branch of `move_to`, `run_transition` | Application `failed_verify_up_retries_only_verifier`, `failed_normal_verify_down_keeps_completed_position_until_retry`, `failed_reverse_verifier_retries_without_mutation_replay_on_either_side`; original two CLI retry tests plus `rollback_verifier_failure_persists_and_retries_only_check_before_walking_farther` | Aligned; call order/counts prove no successful mutation replay. |
| ADR-0002 reverse action; discovery acceptance | Opposite-direction intent runs the same active stage's opposite mutation and optional verifier. Symmetric for pending down. | Application pending branch of `move_to`, `run_transition` | Application `failed_verify_up_backs_out_same_stage_and_can_walk_farther`, `pending_down_reverses_same_stage_and_can_walk_farther_up`; CLI `failed_verify_up_backs_out_same_stage_or_farther_across_cli_processes`, `pending_down_reverses_up_and_reverse_verification_is_resumable_across_processes` | Aligned; abandoned direction's mutation/verifier is not replayed. |
| ADR-0002 migration order; discovery target model | Resolve active stage first, walk each intermediate stage, stop on first failure; settled target no-op; wrong direction executes nothing. | Application `move_to`, `target_count` | Application forward/backward walk, farther reversal, `settled_target_is_noop_and_unreachable_targets_do_not_execute`, `active_unreachable_targets_do_not_execute_or_change_state`, `targets_are_stage_numbers_not_contiguous_numeric_counts`; CLI farther-backout tests | Aligned, including noncontiguous stage numbers and target zero. |
| ADR-0002 failed mutation | Report and stop, no automatic verification, later stage or completion; recovery remains deferred. | Application `run_transition`, `run_role`, `failure_for_last_execution` | Application `mutation_failure_stops_without_verifying_or_advancing_in_both_directions`, `failed_reverse_mutation_stops_and_preserves_previous_active_checkpoint`; CLI `failed_mutation_stops_before_its_verifier_and_later_stages` | Aligned; no cleanup/recovery feature added. |
| ADR-0003 minimal handoff; discovery UUID fixture | Same opaque UUID across all stages, verifiers and downs; keep it while verification is pending; clear only at settled baseline. | Application UUID creation in `run_transition`, handoff in `run_role`, lifetime in `finish_transition`; Infrastructure `SystemExecutableRunner::run` | Application every-role UUID walk and `baseline_rollback_verifier_failure_retains_uuid_until_acceptance`; CLI full traversal logs every role's UUID; manual modified fixture checks all invocation UUIDs | Aligned; new run creates a new UUID after baseline. |
| ADR-0002 session scope; ADR-0004 SQLite | Preserve minimal active state through ordinary CLI process exits, including reverse verification. | Application Query/Write ports; Database `workbench::{SqliteWorkbenchQueries, SqliteWorkbenchWrites}` | CLI normal pending retry, failed-up backout, rollback retry, symmetric pending-down/reverse-up retry; Database `explicit_setup_and_checkpoint_round_trip_use_real_sqlite`, legacy adoption | Aligned through actual separately spawned CLI processes, not merely fakes. |
| ADR-0004/0005 ownership; playbook ledger | Application decides; SQLite maps/stores; CLI parses/renders and explicitly composes. Schema operations remain separate. | Application `Workbench`; Database `src/workbench/mod.rs`, `operations`; CLI `src/commands.rs`, `src/deps.rs` | Source diff/inspection, executed Cargo metadata/tree; CLI `ordinary_cli_does_not_bootstrap_or_migrate`; unchanged typed-source tests and migration tests | Aligned; production change confined to Application. Historical compliance ledger preserved byte-for-byte. |

## Executed verification

Environment: native macOS Unix, Rust `1.94.0 (4a4ef493e 2026-03-02)` and Cargo `1.94.0 (85eff7c80 2026-01-15)`. CLI integrations are Unix-gated and did execute here; these results do not prove other platforms or the declared Rust 1.85 minimum.

| Exact command | Actual result |
| --- | --- |
| `cargo fmt --check` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no warnings. |
| `cargo test --workspace` | Exit 0; 31 passed, 0 failed, 0 ignored. Application 17 (16 semantics + existing error-chain test), CLI 9, Database 4, Infrastructure 1. Empty/doc-test harnesses contain zero tests. |
| `cargo metadata --format-version 1 --no-deps` | Exit 0; same four packages, binary-only CLI, no changed dependency kind/target/feature. New Application integration-test target only. |
| `cargo tree --workspace --edges normal,build,dev --depth 1` | Exit 0; Application has no outer workspace dependency; Database/Infrastructure depend inward on Application; CLI composes all three; existing CLI dev edge to Database remains. |
| `git diff --check` | Exit 0. |

Manual standard fixture: created `/tmp/control-tower-semantics.97qO5x` with `mktemp`, copied `examples/uuid-file/.`, and executed `just db-bootstrap-local`, `just db-migrate-local`, and `just db-verify-local` with that explicit workspace. Each exited 0. Executed separate `cargo run -p control-tower-cli --` processes for status, up to 3, status, down to 1, status, down to 0, status. Each exited 0. Output showed all intermediate mutations and checks, same UUID at stages 3 and 1, and no UUID at settled baseline.

Manual modified fixture: copied the scripts to `/tmp/control-tower-backout-x34cyf_v`, ran the same three supported `just` setup operations (all exit 0), and added call/UUID logging plus one intentional failure to each stage-3 verifier. Separate `target/debug/control-tower` processes produced:

| CLI command (each with `--workspace /tmp/control-tower-backout-x34cyf_v`) | Exit / observed state |
| --- | --- |
| `up --stage 3` | Expected exit 1, stage 3 verify-up exited 23; completed 2, pending up 3. |
| `status` | Exit 0; completed 2, pending up 3, UUID retained. |
| `down --stage 1` | Expected exit 1, stage 3 down succeeded, verify-down exited 23; completed 2, pending down 3. No stage-2 mutation ran. |
| `status` | Exit 0; completed 2, pending down 3, same UUID. |
| `down --stage 1` | Exit 0; only stage-3 verify-down retried, then stage-2 down/verify-down; completed 1, pending clear. |
| `status` | Exit 0; completed 1, same UUID, no pending verification. |
| `down --stage 0` | Exit 0; stage-1 down/verify-down. |
| `status` | Exit 0; baseline, no UUID or pending verification. |

The manual driver asserted every subprocess exit, exact script call order, a single UUID across every logged role, and an empty data directory at baseline. It observed stage-3 up once, stage-3 down once, verify-up once and verify-down twice. Application code never inspected fixture file contents.

Re-read ADR-0002, current design, open questions, discovery record and historical playbook ledger after the fix. Current design's known backout gap is resolved, its stale generalized checkpoint example is replaced with the implemented minimal model, and ADR-0002 gains an evidence link without a change in authority or meaning. Discovery retains its historical baseline and gains a later-evidence link. Failed-mutation recovery, crash reconciliation, concurrency/drift handling and generalized context remain deferred. No code/accepted-document disagreement remains within this run-semantics scope.

The fixture now lives at [examples/simple/uuid-file](../../examples/simple/uuid-file/README.md); the command paths above record the original validation run.
