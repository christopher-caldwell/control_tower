---
id: CT-REF-CLI
title: CLI reference
type: reference
status: maintained
created: '2026-10-01'
updated: '2026-10-06'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../crates/cli/src/main.rs
- ../../crates/cli/src/commands.rs
- ../../crates/cli/src/deps.rs
- ../../crates/cli/src/web.rs
- ../../crates/database/src/operations.rs
- ../../justfile
---

# CLI reference

This page describes the implemented CLI, not proposed command spellings from discovery. The executable name is **`control-tower`** (hyphen), not `control_tower`. The optional browser UI is served by the generic Rust loopback host; see the [browser workbench reference](browser-workbench.md). For first use, follow [setup](../guides/getting-started.md).

## Invocation forms

Workflow-scoped commands require the invocation current directory to be the Workspace root. From that root, you may run the installed `control-tower` binary, invoke a built binary by absolute path, or run Cargo with an absolute manifest path:

```sh
cargo run --locked --manifest-path /path/to/control_tower/Cargo.toml -p control-tower-cli -- status --workflow workflows/my-workflow
```

The package is `control-tower-cli`; its executable is `control-tower`. For database operations:

```sh
cargo run --locked --manifest-path /path/to/control_tower/Cargo.toml -p control-tower-cli -- db verify-local --workflow workflows/my-workflow
```

The invocation current directory identifies the Workspace and must contain a valid `control-tower.toml` with a nonempty `[workspace].label` and a `workflows/` directory. Run each workflow-scoped command from that Workspace root and select the Workflow with `--workflow`; paths such as `workflows/my-workflow` are the normal form.

## Workbench commands

```text
control-tower guide [ACTION]
control-tower init --workflow PATH
control-tower validate --workflow PATH
control-tower ui
control-tower up --workflow PATH --stage NUMBER
control-tower down --workflow PATH --stage NUMBER
control-tower status --workflow PATH
control-tower db bootstrap-local --workflow PATH
control-tower db migrate-local --workflow PATH
control-tower db verify-local --workflow PATH
```

### Agent guidance and validation

`control-tower init --workflow PATH` prepares a new Workflow and checks that it
loads. It runs the existing `db bootstrap-local`, `db migrate-local`, and
`db verify-local` operations in order, then runs `validate`. It stops and exits
with failure at the first failed step. It uses the same Workspace-root `.env`
behavior as `validate`; the database operations do not load `.env`. Use the
individual commands when you need to prepare or inspect storage separately.

`control-tower guide` prints the embedded routing index and does not require a
workflow. Its only actions are `create_workflow`, `edit_workflow`,
`workflow_contract`, `operate_workflow`, and `recover_workflow`; each prints
its compile-time embedded Markdown. The shipped zero-guidance agent Skill is
[`skills/control-tower/SKILL.md`](../../skills/control-tower/SKILL.md) and
dispatches to this CLI guidance.

`control-tower validate --workflow PATH` checks the selected workflow under the
current Workspace root. It validates Workspace configuration and inventory,
discovers stages, reads/checks the saved checkpoint, and parses the Workspace
`.env` if present. It runs no Stage Action and does not prepare or change workflow
storage. It does not probe application/runtime prerequisites or establish that
external application state matches the checkpoint.

`PATH` must be an existing workflow with a prepared database and a `stages/` directory. `NUMBER` is an existing stage's numeric prefix, not a count of commands to execute. `0` denotes the baseline. Stage numbers may have gaps; `--stage 20` selects a stage numbered 20, not the twentieth stage.

| Command | Behavior |
| --- | --- |
| `up` | Apply each needed stage toward the target, in numeric order. Run each supplied `verify-up` before recording that stage complete. |
| `down` | Reverse stages toward the target, in reverse numeric order. To go from stage 3 to stage 2, run stage 3's `down` and supplied `verify-down`; do not run stage 2's `down`. |
| `status` | Read the saved completed position, UUID, pending verification when present, and number of discovered stages. It does not run a verifier or inspect external fixture correctness. |

`ui` starts the loopback browser host from the current directory, which becomes the launch-scoped Workspace. The command prints a plain local URL to open manually. The host uses the committed static React build; Node is not a runtime dependency. The UI offers Run next, Run to a selected farther stage, and Run all, each submitted as one movement request that the Application movement engine walks; selection is primarily inspection, with outcome-specific pending-verification recovery. See [build, workflow inventory, and interface details](browser-workbench.md).

Every stage crossed is completed separately; the walk stops on the first failure. A settled target is a no-op and says that no Stage Actions ran; it does not recheck the fixture. A target in the wrong direction or an unknown stage produces a nonzero result without stage execution.

### Pending verification

If a mutation succeeded but verification failed, the active stage is resolved **before** any further walk:

| Request | Action on the active stage |
| --- | --- |
| Continue the same direction to a reachable target | Retry only the matching verifier; do not replay the mutation. |
| Move the opposite direction to a reachable target | Run the opposite mutation for that same stage, then its optional verifier. |

The completed pointer alone does not make a request a no-op while a transition is pending. For example, completed 2 with pending stage 3/up still requires stage 3/down when requesting `down --stage 2`.

There is no separate `verify` command. Repeat `up` or `down` with the appropriate target to retry a pending verifier. Repeating a command at an already settled target does not reverify it. See [the worked failure example](../guides/verification-and-navigation.md#try-a-verification-failure).

After this invocation fails at a verifier, the CLI prints shell-quoted commands using the current executable and absolute workflow. Their targets resolve **only the active stage**: for stages 10/200, pending 200/up offers up 200 and, if its down exists, down 10. Pending 200/down offers down 10 and, if its up exists, up 200. The first stage's lower target is 0. A farther target can continue traversal after resolving the check. These choices are not printed after a mutation or checkpoint-save failure, even when an older pending checkpoint remains.

### Help, output, and exit results

```sh
control-tower --help
control-tower guide --help
control-tower init --help
control-tower validate --help
control-tower up --help
control-tower down --help
control-tower status --help
control-tower ui --help
control-tower db --help
control-tower db bootstrap-local --help
control-tower db migrate-local --help
control-tower db verify-local --help
```

Control Tower CLI outcomes use these exit codes:

| Exit | Meaning |
| --- | --- |
| `0` | Success or valid no-op. |
| `1` | Operational failure, mutation failure, or executable/verifier start failure. |
| `2` | Invalid CLI invocation (Clap usage error). |
| `3` | A verifier ran and rejected the transition. |

Only an executed verifier that returns failure produces 3. A verifier that cannot start is an operational failure and returns 1. Child-process exit status remains separately visible in Stage Action output and the movement summary; it is not remapped as Control Tower's exit code. Successful guide requests, status, validation, and database operations return 0.

Movement first identifies the selected workflow and requested direction/target. Each actual Stage Action attempt gets a flushed start line with its numeric stage, label and executable name, followed by captured stdout/stderr and success, nonzero exit or launch failure **when that Stage Action returns, before the next is attempted**. Starting means an invocation will be attempted, not that the OS has launched it. An absent optional verifier has no start/result lines. Final output does not replay child output.

Output remains buffered for one Stage Action; a quiet long-running action shows its start but does not stream intermediate bytes. Stdout/stderr stay separate, with stage/executable context and a display newline when needed; the captured bytes in Application remain unchanged. Relative chronology across the streams is not preserved. There is no structured JSON contract or persistent log viewer.

Every movement ends with a compact summary naming the requested target and resulting Control Tower position. When known it includes pending verification, failed Stage Action, and child exit status. It does not parse authored PASS/FAIL output, invent assertion totals, or classify application-level meaning. Final movement output and `status` identify the same recorded completed stage (actual number/label, or baseline 0), run UUID and pending direction/stage. `status` is metadata, not a fresh external-state assertion. A Stage Action's success line confirms process success, not that the subsequent checkpoint save succeeded.

If a checkpoint write fails, output distinguishes the **last confirmed checkpoint** from the **unconfirmed checkpoint update** and retains successful Stage Action output. No further action/write is attempted. Under a fail-before-write fault, a later `status` agrees with the last confirmed checkpoint. An ambiguous storage error carries no guarantee about the database's current contents or external effects; inspect those yourself. There is no automatic write retry or repair.

## Database operations

```text
control-tower db bootstrap-local --workflow PATH
control-tower db migrate-local --workflow PATH
control-tower db verify-local --workflow PATH
```

These require `--workflow PATH`, like every other workflow-scoped command. Run them from the Workspace root and select the Workflow, normally with a path such as `workflows/my-workflow`. All operate on `PATH/.control_tower/state.sqlite3`.

| Operation | Effect |
| --- | --- |
| `bootstrap-local` | Create the local state directory/file if necessary. Do not create application tables or erase existing state. |
| `migrate-local` | Open an existing file and apply the supported versioned schema. The current version-1 migration can adopt the earlier unversioned v0 table without discarding its rows. An already supported version is checked, not reset. |
| `verify-local` | Open read-only and check the supported schema version/history. It does not verify user-authored stages. |

Each successful operation prints a concise confirmation of the operation. A successful operation exits 0; operational failures exit 1 and CLI usage errors exit 2. Ordinary workbench commands never run these operations.

`verify-local` checks version/history, not comprehensive schema readiness. A missing/malformed checkpoint table or unsuitable upsert key can still fail ordinary commands even after version/history verification succeeds. No schema repair is inferred from this operation.

From the repository root, the `justfile` supplies equivalents:

```sh
just db-bootstrap-local "$workflow"
just db-migrate-local "$workflow"
just db-verify-local "$workflow"
```

These recipes call the same operations through Cargo. Their names do not refer to stage `up`/`down` or stage verification.

## Storage and limitations

State is local to each workflow. Keep using the same path across commands. Normal process exits retain the last recorded position, UUID and pending direction. **Restarting does not reset a workflow.**

A successful return to baseline clears the active run UUID but leaves the database prepared. There is no `reset`, `force`, `skip`, arbitrary-action or resume-after-crash command. Directory-structure changes during a stored run, multiple instances and external-state reconciliation are outside v0's guarantees. See [troubleshooting](../guides/troubleshooting.md) instead of deleting SQLite as an attempted external rollback.

## Implementation references

[CLI parser and database dispatch](../../crates/cli/src/main.rs), [delivery/output](../../crates/cli/src/commands.rs), [Application movement](../../crates/application/src/lib.rs), and [database operations](../../crates/database/src/operations.rs) define the current behavior. Accepted navigation intent remains in [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).
