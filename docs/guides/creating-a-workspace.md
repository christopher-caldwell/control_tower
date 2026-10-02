---
id: CT-GUIDE-WORKSPACE
title: Create a workspace
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-02'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../reference/stage-executables.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../../examples/simple/workspaces/uuid-file/README.md
---

# Create a workspace

A workspace is a directory containing ordered executable stages and its own Control Tower database. You edit the scripts with your normal editor; Control Tower runs them.

First [build the two executables](getting-started.md#get-the-source-and-build). Run the commands below from the repository root in one terminal.

## Start with one stage

This new disposable workspace creates a marker, checks it, then removes it. It does not need a UUID or any application-specific adapter.

```sh
workspace="$(mktemp -d)"
stage="$workspace/stages/001-marker"
mkdir -p "$stage"
printf 'New workspace: %s\n' "$workspace"

cat > "$stage/up" <<'SH'
#!/bin/sh
set -eu
printf 'ready' > "$CONTROL_TOWER_WORKSPACE/marker"
SH

cat > "$stage/verify-up" <<'SH'
#!/bin/sh
set -eu
test "$(cat "$CONTROL_TOWER_WORKSPACE/marker")" = 'ready'
SH

cat > "$stage/down" <<'SH'
#!/bin/sh
set -eu
rm "$CONTROL_TOWER_WORKSPACE/marker"
SH

cat > "$stage/verify-down" <<'SH'
#!/bin/sh
set -eu
test ! -e "$CONTROL_TOWER_WORKSPACE/marker"
SH

chmod +x "$stage/up" "$stage/down" "$stage/verify-up" "$stage/verify-down"
```

The quoted heredoc delimiters (`<<'SH'`) keep `$CONTROL_TOWER_WORKSPACE` inside each script for expansion at execution time. They do not substitute it while you create the file.

Both verifiers only observe the result. `up` and `down` perform the changes. The runner cannot enforce that separation, so make it explicit in your scripts.

## Prepare storage and try it

```sh
./target/debug/control-tower-db bootstrap-local "$workspace"
./target/debug/control-tower-db migrate-local "$workspace"
./target/debug/control-tower-db verify-local "$workspace"

./target/debug/control-tower up --workspace "$workspace" --stage 1
cat "$workspace/marker"
printf '\n'
./target/debug/control-tower status --workspace "$workspace"

./target/debug/control-tower down --workspace "$workspace" --stage 0
test ! -e "$workspace/marker"
./target/debug/control-tower status --workspace "$workspace"
```

You should see `ready` after `up`, then baseline 0 after `down`. The application still allocates a run UUID, but your scripts do not have to use it. It is cleared after successful return to baseline.

## Extend the workspace

Add another numbered directory for the next useful state. For example, `002-associated-data` can contain scripts that create related test records, while `003-mutate-data` invokes the application behavior you are debugging.

A stage number identifies a position in the ordered sequence. `up --stage 3` walks through any earlier unapplied stages; it does not jump directly to stage 3. `down --stage 1` reverses stage 3 and then stage 2, leaving stage 1 applied. You do not write pairwise reset scripts for every possible source/target combination.

For a complete concrete example, inspect the checked-in [UUID-file workspace](../../examples/simple/workspaces/uuid-file/README.md). Copy it before editing rather than modifying the acceptance fixture in place.

Use fixed role filenames without extensions. Numbered stages can have gaps, but numeric prefixes must be unique. Put shared support files in a separate workspace directory such as `support/`, not a nonnumeric directory under `stages/`. The [executable reference](../reference/stage-executables.md#workspace-layout) lists exact discovery rules.

## Write ordinary programs

Each role runs from its stage directory. Use the absolute `CONTROL_TOWER_WORKSPACE` variable for data shared across stages; use stage-relative paths for files stored alongside that executable. A shebang selects the interpreter, and you install any interpreter or libraries yourself.

Your scripts can call APIs, use a database client, or invoke another project executable. Return a nonzero exit status when the operation/check should stop the walk; merely printing an error does not fail a role.

`CONTROL_TOWER_UUID` is a convenient run token, not a mechanism for collecting generated API IDs. The runner does not parse stdout into state. Scripts that need richer handoff must explicitly manage their own files for now; see [the current handoff limit](../reference/stage-executables.md#what-the-uuid-does-and-does-not-mean).

For a copyable two-stage pattern, use the optional [application-generated-ID example](../../examples/simple/workspaces/generated-id/README.md). It adds Python 3's standard-library SQLite client: stage 1 creates a row and writes its generated ID to an author-owned JSON file; stage 2 changes/restores the same row. Its DB is separate from `.control_tower/state.sqlite3`. Verifiers use read-only connections, including on missing DB/schema paths, and fail instead of repairing their own assertions. Creation belongs to mutation/setup paths; use ordinary nonzero error handling that still works when interpreter assertions are disabled.

## Iterate without recreating everything

You can edit a role's contents between CLI invocations. If its mutation succeeded and verification failed, repeating the same direction retries only that verifier. Reversing direction runs that active stage's opposite mutation and optional check. Read [navigation and verification](verification-and-navigation.md) before testing those paths.

Role-level starts/results let you see what actually ran. The CLI's failed-verifier choices target only the active stage; a farther target can continue walking after resolution. Removing a pending optional check changes that resolution to acceptance without verification. A settled target and `status` do not check the fixture again.

Do not rename, reorder, insert, or remove stage directories during a stored run and expect Control Tower to reconcile the old position. Finish the run before structural edits, or use a fresh workspace copy with fresh storage after handling external effects yourself. **Restarting the CLI is not a reset.**

Neither a new workspace nor deleting local bookkeeping undoes API/database mutations from the old one. Only your scripts or manual cleanup know how to reverse those effects. There is no automatic reset/cleanup contract in v0.

A failed first mutation may leave partial effects and a recorded UUID at baseline with no pending check. Down 0 is then a no-op, not cleanup. Inspect effects before retrying, since the successful parts may run again with the same UUID. Fixture writes, handoff files and Control Tower checkpoint writes are separate operations, even when the fixture also uses SQLite.

A failed later mutation can likewise remain unaccepted while its external effect
exists; normal traversal does not invoke that stage's `down`. Follow the
[mutation-failure guidance](verification-and-navigation.md#a-mutation-failure-is-not-a-verifier-failure)
and the concrete [Postgres retry/abandon procedure](../../examples/simple/workspaces/postgres/README.md#failed-stage-002-retry-or-abandon)
before returning to baseline. Recovery is owned by the workflow that made the effect.
