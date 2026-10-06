---
id: CT-GUIDE-WORKFLOW
title: Create a workflow
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-05'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../reference/stage-executables.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../../examples/simple/workflows/uuid-file/README.md
---

# Create a workflow

For the complete first-use path, including installation, Workspace setup, database preparation, and the UI/CLI choice, follow [Get started](getting-started.md). This guide adds detail for authors once a Workspace is ready.

A **Workspace** has a `control-tower.toml` with a nonempty `[workspace].label` and a `workflows/` directory. A **Workflow** is a directory beneath `workflows/` with ordered stages and its own Control Tower checkpoint database. Workflow-scoped commands run from the Workspace root and require `--workflow`.

## Design meaningful stages

Before creating files, identify the proposed stage boundaries. A stage should capture one coherent mutation at a meaningful state boundary: a state worth reaching, inspecting, verifying, and where appropriate reversing as a unit. Combine implementation commands that produce one useful state rather than splitting at command boundaries. The [UUID-file example](../../examples/simple/workflows/uuid-file/README.md) shows three meaningful states: create an empty file, write `hello`, then append ` to you`.

## Define each stage's roles

Use numbered directories under `stages/`. Role filenames are the exact protocol: `up`, `down`, `verify-up`, and `verify-down`, with no extension. `up` performs the forward mutation; `down` returns to the prior state or applies an appropriate compensation. `verify-up` and `verify-down` independently observe those outcomes and exit 0 only when accepted. Verifiers are optional and must match an existing mutation. If no compensating action is needed, include an executable `down` that exits 0 without changing state; this explicit no-op lets backward traversal pass the stage.

A role may be any directly executable file supported by the system. Its shebang selects the interpreter/runtime. Supply a valid shebang, execute permission, and the runtime/dependencies yourself. Roles run with the stage directory as their working directory. Use `CONTROL_TOWER_WORKFLOW` for paths shared across stages. The [executable reference](../reference/stage-executables.md) documents numbering, environment values, and process behavior.

## Prepare and validate

Each new Workflow needs explicit database setup. From the Workspace root, replace `NAME` with its path under `workflows/`:

```sh
control-tower db bootstrap-local --workflow workflows/NAME
control-tower db migrate-local --workflow workflows/NAME
control-tower db verify-local --workflow workflows/NAME
control-tower validate --workflow workflows/NAME
```

The three database commands prepare/check checkpoint storage; validation checks discoverability and loadable saved state without running roles. Ordinary movement and status commands do not prepare storage.

A target stage is a position in the ordered sequence. `up --stage 3` walks through all needed earlier stages; it does not run only stage 3. `down --stage 1` reverses higher applied stages until position 1. The [navigation guide](verification-and-navigation.md) describes verifier retry, reversal, and failure behavior.

## Iterate safely

You may edit role contents between CLI invocations. If a mutation succeeded but its verifier failed, repeating that direction retries only the verifier; requesting the opposite direction backs out the active stage. Inspect author-owned state before either action.

Do not rename, reorder, insert, or remove stage directories during a stored run and expect Control Tower to reconcile the old position. Finish or recover the run before structural edits, or use a fresh Workflow copy with fresh storage after handling outside effects. Restarting the CLI or UI does not reconcile structural changes. The UI reads selected Workflow status and stage definitions through its refresh/reconnect path; its Workflow inventory is fixed at host startup, so restart the UI after adding or removing Workflows.

Neither a new Workflow nor deleting local bookkeeping undoes API/database mutations from an old one. A failed mutation may leave partial effects; ordinary traversal does not infer or repair them. Recovery belongs to the Workflow author and the system that owns those effects. See [troubleshooting](troubleshooting.md) and the concrete [Postgres recovery example](../../examples/simple/workflows/postgres/README.md#failed-stage-002-retry-or-abandon).
