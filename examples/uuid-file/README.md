# Three-stage UUID-file example

A runnable workspace and the fixture used by the CLI integration tests. Control Tower supplies one UUID; these scripts create a file named with it and change only its contents. No API, application database, or extra script runtime is required. Control Tower itself still uses SQLite for navigation state.

## Inspect the scripts

```text
stages/
  001-create-file/    up  down  verify-up  verify-down
  002-write-hello/    up  down  verify-up  verify-down
  003-add-to-you/     up  down  verify-up  verify-down
```

| Stage | Forward mutation | Backward mutation | Checks |
| --- | --- | --- | --- |
| [1: create file](stages/001-create-file/up) | Create empty `data/$CONTROL_TOWER_UUID`. | [Delete that file](stages/001-create-file/down). | [Exists](stages/001-create-file/verify-up) / [absent](stages/001-create-file/verify-down). |
| [2: write hello](stages/002-write-hello/up) | Write `hello`. | [Empty the file](stages/002-write-hello/down). | [Contains hello](stages/002-write-hello/verify-up) / [exists and is empty](stages/002-write-hello/verify-down). |
| [3: add suffix](stages/003-add-to-you/up) | Append ` to you`. | [Remove only that trailing suffix](stages/003-add-to-you/down). | [Contains hello to you](stages/003-add-to-you/verify-up) / [back to hello](stages/003-add-to-you/verify-down). |

The sample writes no trailing newline. The content checks use shell command substitution; they are checks for this sample, not a general binary-file comparison utility.

## Build and prepare a copy

From the repository root, with [the prerequisites installed](../../docs/guides/getting-started.md#prerequisites):

```sh
cargo build --locked --workspace
workspace="$(mktemp -d)"
cp -R examples/uuid-file/. "$workspace/"
printf 'Example workspace: %s\n' "$workspace"

./target/debug/control-tower-db bootstrap-local "$workspace"
./target/debug/control-tower-db migrate-local "$workspace"
./target/debug/control-tower-db verify-local "$workspace"
```

Keep the printed path and use this terminal for the remaining steps. In a later terminal, set `$workspace` to that same path. Do not run against the checked-in example directory: the copy keeps generated files and SQLite state out of the acceptance fixture.

## Walk forward and observe

```sh
./target/debug/control-tower status --workspace "$workspace"
./target/debug/control-tower up --workspace "$workspace" --stage 1
./target/debug/control-tower status --workspace "$workspace"
ls -l "$workspace/data"
```

Status starts at baseline, then reports stage 1 and a UUID. `data/` now contains one zero-byte file whose name is that UUID.

```sh
./target/debug/control-tower up --workspace "$workspace" --stage 2
./target/debug/control-tower status --workspace "$workspace"
cat "$workspace"/data/*
printf '\n'

./target/debug/control-tower up --workspace "$workspace" --stage 3
./target/debug/control-tower status --workspace "$workspace"
cat "$workspace"/data/*
printf '\n'
```

The same file contains `hello` at stage 2 and `hello to you` at stage 3. Each command also reports its mutation and verifier results. To run the whole forward sequence in one invocation from baseline, use `up --stage 3`; the intermediate stages are still executed and verified in order.

## Walk backward and observe

```sh
./target/debug/control-tower down --workspace "$workspace" --stage 2
./target/debug/control-tower status --workspace "$workspace"
cat "$workspace"/data/*
printf '\n'

./target/debug/control-tower down --workspace "$workspace" --stage 1
./target/debug/control-tower status --workspace "$workspace"
wc -c "$workspace"/data/*

./target/debug/control-tower down --workspace "$workspace" --stage 0
./target/debug/control-tower status --workspace "$workspace"
ls -A "$workspace/data"
```

Expected observations:

| Completed position | File state | UUID in status |
| --- | --- | --- |
| 2 | `hello` | Same run UUID |
| 1 | Zero bytes | Same run UUID |
| 0 | No file; final `ls` prints nothing | `not created` |

The database and directories remain. They are not deleted by stage 1/down. Another up starts a new run with a new UUID. You can also go `3 -> 2 -> 3` before returning to baseline; the run UUID stays the same.

## Try failed verification

The [navigation guide](../../docs/guides/verification-and-navigation.md#try-a-verification-failure) intentionally breaks a verifier in a temporary copy, then demonstrates retry or same-stage backout. It does not modify these checked-in scripts.

## Evidence and limits

[CLI tests](../../crates/cli/tests/workbench_cli.rs) copy this workspace and spawn real Control Tower processes with SQLite. They check the full traversal, pending verification, reversal, UUID continuity and non-replay of successful mutations. [Application tests](../../crates/application/tests/run_semantics.rs) check the transition rules independently.

The [run-semantics record](../../docs/research/run-semantics-validation.md) records the worker's 31-test macOS run and manual CLI exercises. The [documentation validation](../../docs/research/documentation-validation.md) separately records script-only execution during this documentation pass and the checks that could not be rerun. Do not treat a script-only check as an independent full CLI acceptance run.

For your own operations, start with [creating a workspace](../../docs/guides/creating-a-workspace.md) and the [executable/environment reference](../../docs/reference/stage-executables.md).
