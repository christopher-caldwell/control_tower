# Three-stage UUID-file workflow

A runnable workflow and the fixture used by the CLI integration tests. Control Tower supplies one UUID; these scripts create a file named with it and change only its contents. No API, application database, or extra script runtime is required. Control Tower itself still uses SQLite for navigation state.

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

## Prepare the workflow

Follow the [canonical getting-started walkthrough](../../../../docs/guides/getting-started.md), then run from the copied example root. This shell-only workflow needs no Python or Node setup:

```sh
workflow=workflows/uuid-file
printf 'Example workflow: %s\n' "$workflow"

control-tower db bootstrap-local --workflow "$workflow"
control-tower db migrate-local --workflow "$workflow"
control-tower db verify-local --workflow "$workflow"
```

Use this terminal for the remaining steps. In a later terminal, return to the copied simple example root and set `workflow=workflows/uuid-file` again.

## Walk forward and observe

```sh
control-tower status --workflow "$workflow"
control-tower up --workflow "$workflow" --stage 1
control-tower status --workflow "$workflow"
ls -l "$workflow/data"
```

Status starts at baseline, then reports stage 1 and a UUID. `data/` now contains one zero-byte file whose name is that UUID.

```sh
control-tower up --workflow "$workflow" --stage 2
control-tower status --workflow "$workflow"
cat "$workflow"/data/*
printf '\n'

control-tower up --workflow "$workflow" --stage 3
control-tower status --workflow "$workflow"
cat "$workflow"/data/*
printf '\n'
```

The same file contains `hello` at stage 2 and `hello to you` at stage 3. Each command also reports its mutation and verifier results. To run the whole forward sequence in one invocation from baseline, use `up --stage 3`; the intermediate stages are still executed and verified in order.

## Walk backward and observe

```sh
control-tower down --workflow "$workflow" --stage 2
control-tower status --workflow "$workflow"
cat "$workflow"/data/*
printf '\n'

control-tower down --workflow "$workflow" --stage 1
control-tower status --workflow "$workflow"
wc -c "$workflow"/data/*

control-tower down --workflow "$workflow" --stage 0
control-tower status --workflow "$workflow"
ls -A "$workflow/data"
```

Expected observations:

| Completed position | File state | UUID in status |
| --- | --- | --- |
| 2 | `hello` | Same run UUID |
| 1 | Zero bytes | Same run UUID |
| 0 | No file; final `ls` prints nothing | `not created` |

The database and directories remain. They are not deleted by stage 1/down. Another up starts a new run with a new UUID. You can also go `3 -> 2 -> 3` before returning to baseline; the run UUID stays the same.

## Try failed verification

The [navigation guide](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/verification-and-navigation.md#try-a-verification-failure) intentionally breaks a verifier in a temporary copy, then demonstrates retry or same-stage backout. It does not modify these checked-in scripts.

## Evidence and limits

[CLI tests](https://github.com/christopher-caldwell/control_tower/blob/main/crates/cli/tests/workbench_cli.rs) copy this workflow and spawn real Control Tower processes with SQLite. They check the full traversal, pending verification, reversal, UUID continuity and non-replay of successful mutations. [Application tests](https://github.com/christopher-caldwell/control_tower/blob/main/crates/application/tests/run_semantics.rs) check the transition rules independently.

The [run-semantics record](https://github.com/christopher-caldwell/control_tower/blob/main/docs/research/run-semantics-validation.md) records the worker's 31-test macOS run and manual CLI exercises. The [documentation validation](https://github.com/christopher-caldwell/control_tower/blob/main/docs/research/documentation-validation.md) separately records script-only execution during this documentation pass and the checks that could not be rerun. Do not treat a script-only check as an independent full CLI acceptance run.

For your own operations, start with [creating a workflow](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/creating-a-workflow.md) and the [executable/environment reference](https://github.com/christopher-caldwell/control_tower/blob/main/docs/reference/stage-executables.md).
