---
id: CT-GUIDE-SETUP
title: Get started with Control Tower
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-05'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../README.md
- ../../examples/simple/workflows/uuid-file/README.md
- ../../crates/cli/guides/workflow_contract.md
- ../reference/browser-workbench.md
- ../reference/stage-executables.md
---

# Get started with Control Tower

This is the canonical end-to-end human walkthrough. It builds a small UUID-file Workspace and Workflow from an empty directory, then lets you choose either the browser UI or CLI to operate it. Commands assume a Unix shell.

## 1. Install Control Tower

You need Git, a current stable Rust toolchain with Cargo, a working native C compiler/linker (the Rust SQLite dependency builds bundled SQLite), Node.js with the package manager version pinned in `ui/package.json`, and `just` only if you use that installer. Neither a separate SQLite installation nor Node at runtime is required. The Rust executable embeds the browser UI, so generate those assets once before either Rust installation path.

Clone the source and build the UI assets:

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
pnpm --dir ui install --frozen-lockfile
pnpm --dir ui build
```

`ui/dist/` is generated and ignored by Git. A fresh clone needs it before the Rust CLI can compile. With `just` installed, install `control-tower` and the shipped agent Skill:

```sh
just install-cli
```

Or install only the CLI with Cargo:

```sh
cargo install --locked --path crates/cli --bin control-tower
```

Ensure Cargo's install directory (normally `~/.cargo/bin`) is on `PATH`. Confirm installation:

```sh
control-tower --help
```

## 2. Create a Workspace and Workflow

Create the small Workspace from scratch. These are the files you own; Control Tower will create its checkpoint database when you prepare the Workflow.

```sh
workspace="$(mktemp -d)/uuid-workspace"
workflow="$workspace/workflows/uuid-file"
mkdir -p "$workflow/stages/001-create-file" "$workflow/stages/002-write-hello" "$workflow/stages/003-add-to-you"
cat > "$workspace/control-tower.toml" <<'EOF'
[workspace]
label = "UUID walkthrough"
EOF
```

The layout is **Workspace → Workflow → Stage → role files**. The Workspace root contains `control-tower.toml` and `workflows/`. Each Workflow is an ordered scenario with its own checkpoint database. A Stage is one meaningful state transition.

Stage roles have exact names and no filename extension. `up` applies a transition, `down` restores the prior state or compensates for it, `verify-up` independently observes the accepted forward state, and `verify-down` observes the restored state. Each role is an executable file; the shebang selects its interpreter. These three stages move one UUID-named file from absent, to empty, to `hello`, then to `hello to you`.

```sh
cat > "$workflow/stages/001-create-file/up" <<'EOF'
#!/bin/sh
set -eu
directory="$CONTROL_TOWER_WORKFLOW/data"
mkdir -p "$directory"
: > "$directory/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/001-create-file/verify-up" <<'EOF'
#!/bin/sh
set -eu
test -f "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
test ! -s "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/001-create-file/down" <<'EOF'
#!/bin/sh
set -eu
rm "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/001-create-file/verify-down" <<'EOF'
#!/bin/sh
set -eu
test ! -e "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF

cat > "$workflow/stages/002-write-hello/up" <<'EOF'
#!/bin/sh
set -eu
printf 'hello' > "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/002-write-hello/verify-up" <<'EOF'
#!/bin/sh
set -eu
test "$(cat "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID")" = hello
EOF
cat > "$workflow/stages/002-write-hello/down" <<'EOF'
#!/bin/sh
set -eu
: > "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/002-write-hello/verify-down" <<'EOF'
#!/bin/sh
set -eu
test -f "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
test ! -s "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF

cat > "$workflow/stages/003-add-to-you/up" <<'EOF'
#!/bin/sh
set -eu
printf ' to you' >> "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/003-add-to-you/verify-up" <<'EOF'
#!/bin/sh
set -eu
test "$(cat "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID")" = 'hello to you'
EOF
cat > "$workflow/stages/003-add-to-you/down" <<'EOF'
#!/bin/sh
set -eu
printf 'hello' > "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID"
EOF
cat > "$workflow/stages/003-add-to-you/verify-down" <<'EOF'
#!/bin/sh
set -eu
test "$(cat "$CONTROL_TOWER_WORKFLOW/data/$CONTROL_TOWER_UUID")" = hello
EOF

chmod +x "$workflow"/stages/*/*
printf 'Workspace: %s\nWorkflow: %s\n' "$workspace" "$workflow"
```

`CONTROL_TOWER_WORKFLOW` names this Workflow's directory, and `CONTROL_TOWER_UUID` is the run's shared UUID. The Workflow owns its stage directories and executable role files. The UUID-named data file is application/test state owned by the Workflow; `.control_tower/state.sqlite3` is separate generated Control Tower checkpoint state.

## 3. Prepare and validate the Workflow

Every Workflow has a separate checkpoint database. Prepare it explicitly before use:

```sh
(cd "$workspace" && control-tower db bootstrap-local --workflow workflows/uuid-file)
(cd "$workspace" && control-tower db migrate-local --workflow workflows/uuid-file)
(cd "$workspace" && control-tower db verify-local --workflow workflows/uuid-file)
(cd "$workspace" && control-tower validate --workflow workflows/uuid-file)
```

Database preparation does not run stages. Ordinary `up`, `down`, and `status` commands do not bootstrap or migrate storage. `validate` checks that the Workspace, Workflow layout, and prepared checkpoint state load; it does not run roles or prove their application-specific behavior.

## 4. Authoring model: meaningful stages and verifiers

A stage should represent one coherent mutation that is useful to run, inspect, verify, and, where appropriate, reverse as a unit. Split stages at meaningful state boundaries, not at implementation-command boundaries. A stage may run several commands to create one useful state; avoid making a stage for each shell command.

A verifier is an executable observation used to decide whether the state required at that transition is acceptable. Exit 0 means the check passed; nonzero rejects the transition. `verify-up` and `verify-down` correspond to the current transition, but may re-check earlier invariants that still need to hold. For example, a later stage may check that a file established several stages earlier still exists. Control Tower does not provide a standalone command to rerun an arbitrary earlier stage's verifier.

For an HTTP-backed workflow, `up` might POST to create state and `verify-up` might GET and assert that state. `down` might DELETE it, and `verify-down` might GET and assert it is gone, such as by observing an expected 404. These illustrate the observation model; they are not required HTTP implementations. Verifiers are optional. If no compensation is needed for a direction, an executable `down` that exits 0 without changing anything is an explicit no-op that lets backward traversal pass that stage.

## 5. Choose how to operate

The UI and CLI operate the same prepared Workflow. Choose either branch.

### UI branch

From the Workspace root, start the UI and open the local URL it prints:

```sh
(cd "$workspace" && control-tower ui)
```

Select `uuid-file`, inspect its stages, then use the movement controls to run to stage 3. Keep the UI host running. The UI lists Workflows found at startup; restart the UI host after adding or removing a Workflow. Selecting the current Workflow re-reads its status and stages, but a structural stage edit during a stored run is not reconciled as a generic reload. Finish the run before changing stage structure.

### CLI branch

From the Workspace root, inspect status and run through stage 3:

```sh
(cd "$workspace" && control-tower status --workflow workflows/uuid-file)
(cd "$workspace" && control-tower up --workflow workflows/uuid-file --stage 3)
(cd "$workspace" && control-tower status --workflow workflows/uuid-file)
```

Status should show stage 3 completed. A target stage means “walk through all needed stages up to this number,” not “run only that stage.”

## 6. Inspect the result and return to baseline

Both surfaces use the same ordered movement behavior. The UUID-file stages progressively take the data file from absent, to empty, to `hello`, to `hello to you`. Inspect the fixture from the shell after either branch:

If you chose the UI branch, leave its host running and use a second terminal for the commands below. Set `workflow` to the path printed after creating the Workspace; set `workspace` to its parent directory above `workflows/` if you need the CLI commands in this section.

```sh
cat "$workflow"/data/*
printf '\n'
```

The file should contain `hello to you`. Then return to baseline using the same surface:

- In the UI, choose the backward movement action repeatedly until the displayed position is baseline 0.
- In the CLI, run these commands from the Workspace root:

  ```sh
  (cd "$workspace" && control-tower down --workflow workflows/uuid-file --stage 0)
  (cd "$workspace" && control-tower status --workflow workflows/uuid-file)
  ```

Then inspect the data directory from the shell, whichever surface you used:

```sh
ls -A "$workflow/data"
```

The data directory is empty and the UI/CLI status reports baseline with no active UUID. The prepared database remains, so you can run the Workflow again. A verifier failure leaves a transition pending; inspect the resulting state before retrying the check or backing out. The [navigation guide](verification-and-navigation.md) explains retry and recovery behavior.

## 7. Keep working safely

Role scripts run with your permissions and may change real systems. Control Tower records workflow progress; it cannot guarantee that `down` undoes `up`, sandbox scripts, or reconcile outside effects after a crash. Finish or explicitly recover a run before structural stage edits. The UI's Workflow inventory is fixed at startup, while selecting a Workflow re-reads its status and stage definitions; neither UI restart nor refresh reconciles structural edits with stored run state.

For deeper authoring guidance see [creating a Workflow](creating-a-workflow.md); for UI details, [browser workbench](../reference/browser-workbench.md); for commands, [CLI reference](../reference/cli.md). The existing [UUID-file example](../../examples/simple/workflows/uuid-file/README.md) is an optional finished reference and ready-made shortcut; it is not required by this walkthrough.
