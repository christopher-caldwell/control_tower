---
id: CT-REF-EXECUTABLES
title: Stage executables and environment
type: reference
status: maintained
created: '2026-10-01'
updated: '2026-10-05'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../crates/infrastructure/src/stage_discovery.rs
- ../../crates/infrastructure/src/executable_runner.rs
- ../../crates/application/src/lib.rs
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
---

# Stage executables and environment

For a complete first-use path and the stage-authoring mental model, see the
[canonical getting-started walkthrough](../guides/getting-started.md).

Control Tower discovers files and runs them. It does not interpret your SQL, HTTP responses, application records, or test assertions. Commands run from the Workspace root, whose `control-tower.toml` must contain a nonempty `[workspace].label` and whose `workflows/` directory is required. Workflow-scoped CLI commands select a workflow with `--workflow PATH`.

## Workflow layout

```text
my-workflow/
  stages/
    001-create-fixture/
      up
      down
      verify-up
      verify-down
    020-mutate-fixture/
      up
      down
  .control_tower/
    state.sqlite3
```

Only numbered directories immediately under `stages/` are stages. Names begin with a positive decimal integer, optionally followed by `-` and a label. `001-create-fixture` and `20` are valid; `0`, duplicate numeric prefixes, nonnumeric stage directories and prefixes beyond the CLI's `u32` range are rejected. Numbers need not be contiguous. Non-directory entries under `stages/` are ignored; keep support directories outside `stages/` so they are not mistaken for stages.

The role filenames are exact: `up`, `down`, `verify-up`, `verify-down`. A file called `up.sh` is not an `up` role. No YAML/TOML configuration or `actions/` directory is used by v0.

## Executable roles

| File | Responsibility |
| --- | --- |
| `up` | Establish this stage's forward state. |
| `verify-up` | Check whether this stage's `up` worked; exit 0 for acceptance. Optional. |
| `down` | Establish the prior stage's state using your chosen reverse/compensating operation. |
| `verify-down` | Check whether this stage's `down` worked; exit 0 for acceptance. Optional. |

Each stage must contain at least one mutation (`up` or `down`). A verifier requires its matching mutation file. A missing mutation makes traversal in that direction unavailable. Missing verifiers need no placeholder: successful mutation is sufficient when that directional verifier is absent.

Role paths must be regular files; execute permission is required when a role is launched. Discovery/status can still succeed when an execute bit is absent; a role path that is a directory fails discovery. For scripts, supply a valid shebang and install the interpreter yourself:

```sh
#!/bin/sh
set -eu
# Your operation here.
```

After writing a role file, make it executable, for example:

```sh
chmod +x "$workflow/stages/001-create-fixture/up"
```

Control Tower starts the file directly; it does not prepend `sh`, install packages, transpile TypeScript, or substitute expressions into its contents. A Node or Python script can use its normal shebang, but those runtimes and libraries remain the author's responsibility.

## Working directory and process behavior

Each role runs with **its own stage directory** as the working directory, not the repository root or workflow root. Use `CONTROL_TOWER_WORKFLOW` for workflow-wide paths. Keep shared helper files outside `stages/`, for example `support/`, and reference them explicitly.

The runner adds no command-line arguments to a role. Its current implementation captures stdout/stderr and gives the child no interactive stdin. Write noninteractive scripts; do not depend on a terminal prompt. Other environment variables are inherited from the launching process, with the five variables below set by Control Tower.

The runner inherits the launching process environment. If `<workspace>/.env` exists, its parsed values fill keys absent from the inherited environment. Shell values take precedence, and Control Tower's `CONTROL_TOWER_*` variables take precedence over both. This is the only automatic dotenv location; no workflow or parent-directory search occurs. Missing `.env` is valid; unreadable or malformed `.env` blocks validation and movement before roles run. `validate --workflow PATH` checks its syntax without running roles.

The CLI flushes the stage/role identity before attempting invocation. It shows captured output and the result when that role returns, before attempting the next. Bytes are buffered for one role, with stdout/stderr displayed separately and a framing newline if a stream lacks one. Application retains the original bytes; stdout is never parsed into state. Ordering between the two streams is not a chronological log. Each movement ends with a concise summary of the requested target, resulting Control Tower position and, when known, pending verification, failed role and child exit status. The child exit status is separate from Control Tower's CLI exit code.

| Variable | Value |
| --- | --- |
| `CONTROL_TOWER_WORKFLOW` | Canonical absolute path to the selected workflow. |
| `CONTROL_TOWER_UUID` | One runner-generated UUID for the active run, reused across stages and directions. |
| `CONTROL_TOWER_STAGE` | Numeric stage identifier without filename padding, such as `3`. |
| `CONTROL_TOWER_DIRECTION` | `up` or `down` for this invocation. |
| `CONTROL_TOWER_ROLE` | `up`, `down`, `verify-up`, or `verify-down`. |

A verifier receives the direction it verifies (`up` for `verify-up`, `down` for `verify-down`). An explicit reverse action receives the new direction and same run UUID.

### What the UUID does and does not mean

**The current implementation generates and records the UUID in Control Tower before the first mutation of a run. The stage does not generate or publish it.** The UUID-file example uses that UUID as a filename. After the workflow successfully records settlement at baseline 0, the UUID is cleared; a later run gets a new one. It remains available while verification of a return to baseline is pending. A failed first mutation can retain the UUID with baseline/no pending check; a down-0 no-op does not clear it, and retry reuses it.

This proves a minimal runner-supplied identifier handoff. There is no supported `WB_OUTPUT`, `DAGU_OUTPUT_FILE`, generic context-patch channel, or parser that turns script stdout into persisted values. A script printing an API-created ID does not automatically pass it to the next stage. Such scripts can deliberately share their own workflow files, but that storage/cleanup is author-owned, not a managed Control Tower output protocol.

Earlier discovery explored stage-produced IDs and richer context views. Those are not the shipped process interface. See [ADR-0003's implementation note](../decisions/0003-session-state-and-process-io.md#current-implementation-observation).

The optional [generated-ID example](../../examples/simple/workflows/generated-id/README.md) uses Python's standard-library SQLite client and an author-owned JSON file to carry the database-generated record ID through two stages. Its application database is separate from Control Tower's checkpoint file. Its checks open read-only connections and never initialize or repair the fixture. This is an ordinary authoring pattern, not a new runner protocol.

## Success, failure, and trust

An exit code of 0 accepts a role. A nonzero mutation stops the walk before its verifier or later stages. A successful mutation followed by failed verification remains unfinished; [navigation](../guides/verification-and-navigation.md) describes explicit retry and reversal.

Write fixture verifiers to observe, not repair, the state they check. Control Tower cannot enforce that convention. Likewise, `down` is not guaranteed undo: any side effects and compensating operations belong to the script author.

Optional roles are discovered again on each invocation. Editing a pending check changes the next check; removing it can accept the pending transition without replaying its mutation or checking anything. Finish/clean runs before structural stage edits. A later missing mutation stops traversal after earlier stages may already have been accepted; the route is not atomic or fully preflighted.

Scripts run with your user permissions and inherited environment, including any credentials you supply. Local-only does not make execution sandboxed or offline. Read scripts before running an unfamiliar workflow.

## Implementation references

[Discovery](../../crates/infrastructure/src/stage_discovery.rs), [process runner](../../crates/infrastructure/src/executable_runner.rs), and [UUID/transition lifecycle](../../crates/application/src/lib.rs) define these mechanics. For a runnable example, open [the UUID-file workflow](../../examples/simple/workflows/uuid-file/README.md).
