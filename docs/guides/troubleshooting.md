---
id: CT-GUIDE-TROUBLESHOOTING
title: Troubleshoot local setup and runs
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-06'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../crates/cli/src/main.rs
- ../../crates/database/src/operations.rs
- ../../crates/infrastructure/src/stage_discovery.rs
- ../reference/cli.md
- ../reference/stage-executables.md
---

# Troubleshoot local setup and runs

Start with the [setup guide](getting-started.md). Build/install commands run from the checkout root; every workflow-scoped CLI invocation runs with current directory set to the Workspace root. The examples below assume `$workflow` still contains the path printed when you created it. From the checkout root, you can define this helper for a built CLI:

```sh
repo="$PWD"
workspace="$(dirname "$(dirname "$workflow")")"
ct() (cd "$workspace" && "$repo/target/debug/control-tower" "$@")
```

## Cargo or a built binary is missing

Check `cargo --version`. Install Rust through the [official rustup instructions](https://rust-lang.org/tools/install/) and reopen the terminal if Cargo is not yet on `PATH`.

Build the CLI:

```sh
cargo build --locked --workspace
```

Use `./target/debug/control-tower` for all commands with Cargo's default target directory. A custom target directory changes that path. A missing native compiler/linker is a Rust/bundled-SQLite build prerequisite, not a reason to install a SQLite server.

The Cargo package is **`control-tower-cli`**, not `control-tower`:

```sh
(cd "$workspace" && cargo run --locked --manifest-path "$repo/Cargo.toml" -p control-tower-cli -- status --workflow "$workflow")
```

`just: command not found` is not a blocker. For normal first use, follow the
setup guide's `control-tower init --workflow "$workflow"` command. Use the
individual database commands below to isolate or diagnose a setup step.

## The database is missing or not initialized

Check that the workflow exists and that you selected the intended path:

```sh
printf 'Workflow: %s\n' "$workflow"
ls -ld "$workflow"
```

For a new workflow, start with the convenience setup command:

```sh
ct init --workflow "$workflow"
```

It runs bootstrap, migration, schema verification, and Workflow validation in
order, and stops at the first failure. If you want to isolate or diagnose one
operation, the underlying commands remain available individually:

```sh
ct db bootstrap-local --workflow "$workflow"
ct db migrate-local --workflow "$workflow"
ct db verify-local --workflow "$workflow"
```

Normal commands intentionally refuse an unprepared database. Bootstrap does not migrate, and ordinary `status` does not create storage. The CLI's startup message suggests setup for database-opening failures in general; if setup/verification already succeeds, inspect the underlying message and filesystem permissions rather than assuming another migration fixes it.

`verify-local` checks schema version/history, not all checkpoint-table columns or constraints. Missing/malformed state tables and unsuitable upsert keys may fail later operations. Preserve the original error rather than treating a successful version/history check as full database readiness.

Do not delete the database to repair external test data. Deleting bookkeeping does not reverse anything a script did. For a damaged/unsupported local database, v0 has no automatic repair contract; preserve what you need and use a fresh workflow after author-owned cleanup.

## No stages are found, or a stage directory is rejected

The actual directory is `stages/`, not the `steps/` name used in older design probes. Each immediate stage directory needs a unique positive numeric prefix and at least an `up` or `down` file. Put support directories elsewhere.

Stage Action filenames have no extension: use `up`, not `up.sh`. A verifier without its matching mutation is rejected. Check [the layout reference](../reference/stage-executables.md#workflow-layout).

`status` also discovers the layout, so a Stage Action path turned into a directory can block status. Removing an execute bit alone does not block discovery/status; it fails when that Stage Action is launched. There is no degraded-status mode for invalid layouts.

## A script could not start

Read the stage and Stage Action identified in the error. Check that the file is executable and its shebang points to an installed interpreter. For a copied/new stage, for example:

```sh
chmod +x "$workflow/stages/001-create-file/up"
```

Apply permissions only to the Stage Actions actually present in your stage. Control Tower directly launches Stage Actions and does not install interpreters. Supply a shebang even if executable shell text happens to work on your host: shebangless text ran in the archived macOS experiment, but its fallback mechanism and other hosts were not verified. A missing interpreter or CRLF shebang can produce “No such file or directory” even when the Stage Action exists. Relative paths inside a script resolve from the **stage directory**; workflow-wide paths should use `CONTROL_TOWER_WORKFLOW`.

The runner gives scripts no interactive stdin. Scripts that prompt for confirmation or credentials need to be made noninteractive. A flushed start line appears before invocation is attempted; captured output/result appears when that Stage Action returns, before the next starts. A long-running Stage Action can remain quiet after its start line because intermediate bytes are not streamed. Output is not persisted as execution history; capture terminal output or use author-owned logs if needed. Signal failures currently lack specific signal identity.

## Verification failed

The final movement output already distinguishes the recorded completed stage from a pending check; `status` can read that bookkeeping again. Use the printed retry command to run only the active verifier, or the printed backout command to run the opposite mutation/check. A farther reachable target first resolves the active stage and can then execute additional stages. Editing/removing an optional pending check changes what runs next.

Follow [the worked failure exercise](verification-and-navigation.md#try-a-verification-failure). Do not repeatedly rerun the mutation manually when a verifier retry is what you need. There is no separate `verify`, `force` or `skip` subcommand.

## A Stage Action succeeded but the checkpoint save failed

The CLI reports the last confirmed checkpoint separately from the unconfirmed attempted update and stops all further work. Successful Stage Action output remains visible. If the pending-marker save failed, the last confirmed state can have a UUID but no pending check: retry may repeat the mutation. If final acceptance failed, the confirmed state can remain pending even after a successful verifier. Inspect external effects before choosing movement; no safe verifier-only recipe is inferred from a save failure or an older pending field after a failed reverse mutation. A later status agrees under deterministic fail-before-write faults, but an ambiguous storage error is not proof of the database's current contents.

## Printing an ID did not pass it to the next stage

That is not an implemented output channel. `CONTROL_TOWER_UUID` is generated by the runner; stdout is a result for the user. The UUID-file example needs only that shared token. Additional script-produced values require your own explicit file handoff; the optional [generated-ID example](../../examples/simple/workflows/generated-id/README.md) demonstrates that pattern with Python and a separate application DB. See [the executable contract](../reference/stage-executables.md#what-the-uuid-does-and-does-not-mean).

## Restarting did not reset the run

That is expected: SQLite persists state. A successful `down --stage 0` runs the authored reversals and settles at baseline. It is not guaranteed cleanup when scripts are broken or the recorded position disagrees with external state.

A failed first mutation can leave partial effects, baseline, a retained UUID and no pending verifier. Down 0 is then a no-op and does not clear the UUID or undo those effects. Inspect/clean up the author-owned fixture yourself before choosing a retry, which reuses that UUID. `status` and a repeated settled target are not fresh fixture checks.

For experiments that cannot be backed out, handle external effects yourself and start in a fresh workflow copy. Changing stage-directory structure during a stored run is unsupported; restarting alone does not reconcile it. Keep the old workflow/identifiers until you no longer need them for cleanup.

Interruption behavior depends on signal delivery. The archived parent-only SIGTERM allowed a later child write; process-group SIGINT stopped it in that experiment. Both retained the preallocated UUID. This does not establish universal Ctrl-C behavior, and v0 provides no process-tree cancellation or crash-recovery guarantee. The [findings register](../research/2026-10-01-guided-usage-findings.md) retains these platform/evidence limits and the deferred observations.
