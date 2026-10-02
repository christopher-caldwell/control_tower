---
id: CT-REF-CLI
title: CLI reference
type: reference
status: maintained
created: '2026-10-01'
updated: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../crates/cli/src/main.rs
- ../../crates/cli/src/commands.rs
- ../../crates/cli/src/deps.rs
- ../../crates/database/src/bin/control-tower-db.rs
- ../../crates/database/src/operations.rs
- ../../justfile
---

# CLI reference

This page describes the implemented CLI, not proposed command spellings from discovery. The executable name is **`control-tower`** (hyphen), not `control_tower`. For first use, follow [setup](../guides/getting-started.md).

## Invocation forms

Commands below use installed binaries. From the repository root, you can instead use `./target/debug/control-tower` after building, or invoke the same command through Cargo:

```sh
cargo run --locked -p control-tower-cli -- status --workspace "$workspace"
```

The package is `control-tower-cli`; its executable is `control-tower`. For database operations:

```sh
cargo run --locked -p control-tower-database --bin control-tower-db -- verify-local "$workspace"
```

An installed binary can run from any directory. Relative workspace paths are resolved against the directory from which you invoke the command. Use an absolute path when changing directories between invocations.

## Workbench commands

```text
control-tower up --workspace PATH --stage NUMBER
control-tower down --workspace PATH --stage NUMBER
control-tower status --workspace PATH
```

`PATH` must be an existing workspace with a prepared database and a `stages/` directory. `NUMBER` is an existing stage's numeric prefix, not a count of commands to execute. `0` denotes the baseline. Stage numbers may have gaps; `--stage 20` selects a stage numbered 20, not the twentieth stage.

| Command | Behavior |
| --- | --- |
| `up` | Apply each needed stage toward the target, in numeric order. Run each supplied `verify-up` before recording that stage complete. |
| `down` | Reverse stages toward the target, in reverse numeric order. To go from stage 3 to stage 2, run stage 3's `down` and supplied `verify-down`; do not run stage 2's `down`. |
| `status` | Read the saved completed position, UUID, pending verification when present, and number of discovered stages. It does not run a verifier or inspect external fixture correctness. |

Every stage crossed is completed separately; the walk stops on the first failure. A settled target is a no-op and says that no roles ran; it does not recheck the fixture. A target in the wrong direction or an unknown stage produces a nonzero result without stage execution.

### Pending verification

If a mutation succeeded but verification failed, the active stage is resolved **before** any further walk:

| Request | Action on the active stage |
| --- | --- |
| Continue the same direction to a reachable target | Retry only the matching verifier; do not replay the mutation. |
| Move the opposite direction to a reachable target | Run the opposite mutation for that same stage, then its optional verifier. |

The completed pointer alone does not make a request a no-op while a transition is pending. For example, completed 2 with pending stage 3/up still requires stage 3/down when requesting `down --stage 2`.

There is no separate `verify` command. Repeat `up` or `down` with the appropriate target to retry a pending verifier. Repeating a command at an already settled target does not reverify it. See [the worked failure example](../guides/verification-and-navigation.md#try-a-verification-failure).

After this invocation fails at a verifier, the CLI prints shell-quoted commands using the current executable and absolute workspace. Their targets resolve **only the active stage**: for stages 10/200, pending 200/up offers up 200 and, if its down exists, down 10. Pending 200/down offers down 10 and, if its up exists, up 200. The first stage's lower target is 0. A farther target can continue traversal after resolving the check. These choices are not printed after a mutation or checkpoint-save failure, even when an older pending checkpoint remains.

### Help, output, and exit results

```sh
control-tower --help
control-tower up --help
control-tower down --help
control-tower status --help
```

A completed/no-op movement, successful status, or workbench help request exits successfully. A stopped movement, failed verifier, failed script start, or workbench error returns a nonzero exit. The child process's own exit code is reported in text; it is not used as the workbench's exit code.

Movement first identifies the selected workspace and requested direction/target. Each actual role attempt gets a flushed start line with its numeric stage, label and role, followed by captured stdout/stderr and success, nonzero exit or launch failure **when that role returns, before the next role is attempted**. Starting means an invocation will be attempted, not that the OS has launched it. An absent optional verifier has no start/result lines. Final output does not replay child output.

Output remains buffered for one role; a quiet long-running role shows its start but does not stream intermediate bytes. Stdout/stderr stay separate, with stage/role context and a display newline when needed; the captured bytes in Application remain unchanged. Relative chronology across the streams is not preserved. There is no structured JSON contract or persistent log viewer.

Final movement output and `status` identify the same recorded completed stage (actual number/label, or baseline 0), run UUID and pending direction/stage. `status` is metadata, not a fresh external-state assertion. A role's success line confirms process success, not that the subsequent checkpoint save succeeded.

If a checkpoint write fails, output distinguishes the **last confirmed checkpoint** from the **unconfirmed checkpoint update** and retains successful role output. No further role/write is attempted. Under a fail-before-write fault, a later `status` agrees with the last confirmed checkpoint. An ambiguous storage error carries no guarantee about the database's current contents or external effects; inspect those yourself. There is no automatic write retry or repair.

## Database operations

```text
control-tower-db bootstrap-local PATH
control-tower-db migrate-local PATH
control-tower-db verify-local PATH
```

These use a positional workspace path, **not** `--workspace`. The path must already be a directory. All operate on `PATH/.control_tower/state.sqlite3`.

| Operation | Effect |
| --- | --- |
| `bootstrap-local` | Create the local state directory/file if necessary. Do not create application tables or erase existing state. |
| `migrate-local` | Open an existing file and apply the supported versioned schema. The current version-1 migration can adopt the earlier unversioned v0 table without discarding its rows. An already supported version is checked, not reset. |
| `verify-local` | Open read-only and check the supported schema version/history. It does not verify user-authored stages. |

Ordinary workbench commands never run these operations. A successful database operation exits 0; usage/setup failures exit nonzero. `control-tower-db` has a minimal positional interface, not the workbench's Clap help interface; running it without the two arguments prints usage and exits nonzero.

`verify-local` checks version/history, not comprehensive schema readiness. A missing/malformed checkpoint table or unsuitable upsert key can still fail ordinary commands even after version/history verification succeeds. No schema repair is inferred from this operation.

From the repository root, the `justfile` supplies equivalents:

```sh
just db-bootstrap-local "$workspace"
just db-migrate-local "$workspace"
just db-verify-local "$workspace"
```

These recipes call the same operations through Cargo. Their names do not refer to stage `up`/`down` or stage verification.

## Storage and limitations

State is local to each workspace. Keep using the same path across commands. Normal process exits retain the last recorded position, UUID and pending direction. **Restarting does not reset a workspace.**

A successful return to baseline clears the active run UUID but leaves the database prepared. There is no `reset`, `force`, `skip`, arbitrary-action or resume-after-crash command. Directory-structure changes during a stored run, multiple instances and external-state reconciliation are outside v0's guarantees. See [troubleshooting](../guides/troubleshooting.md) instead of deleting SQLite as an attempted external rollback.

## Implementation references

[Workbench parser](../../crates/cli/src/main.rs), [delivery/output](../../crates/cli/src/commands.rs), [Application movement](../../crates/application/src/lib.rs), [database operations](../../crates/database/src/operations.rs), and [operational parser](../../crates/database/src/bin/control-tower-db.rs) define the current behavior. Accepted navigation intent remains in [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).
