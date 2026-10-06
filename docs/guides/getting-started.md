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

This is the canonical end-to-end human walkthrough. It uses the existing simple UUID-file example and lets you choose either the browser UI or CLI for operation. Commands assume a Unix shell.

## 1. Install Control Tower

You need Git, a current stable Rust toolchain with Cargo, and a working native C compiler/linker (the Rust SQLite dependency builds bundled SQLite). Neither `just` nor a separate SQLite installation is required when using the Cargo path. The browser UI is included in the Rust executable; Node is not a runtime requirement.

Clone the source:

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
```

Install `control-tower` onto your `PATH` with either option:

```sh
just install-cli
```

This `just` recipe also copies the shipped agent Skill to `~/.agents/skills`; it requires `just`. Or install only the CLI with Cargo:

```sh
cargo install --locked --path crates/cli --bin control-tower
```

Ensure Cargo's install directory (normally `~/.cargo/bin`) is on `PATH`. Confirm installation:

```sh
control-tower --help
```

## 2. Understand the Workspace

The hierarchy is **Workspace → Workflow → Stage → role files**. The directory where you run the UI or a workflow-scoped CLI command is the Workspace root. It contains `control-tower.toml` and `workflows/`. Each Workflow under `workflows/` is an ordered scenario with its own stages and checkpoint database. A Stage is one meaningful state transition in that scenario.

For orientation, the copied example has this shape:

```text
simple/                              # Workspace; author-owned
  control-tower.toml                 # Workspace label/configuration
  workflows/                         # Workflow inventory
    uuid-file/                       # one Workflow; author-owned
      README.md                      # example instructions
      stages/                        # ordered Stage definitions
        001-create-file/             # one meaningful state change
          up                          # apply this transition
          down                        # restore the prior state
          verify-up                   # independently check up (optional)
          verify-down                 # independently check down (optional)
        002-write-hello/
        003-add-to-you/
      data/                           # example's author-owned fixture data
      .control_tower/state.sqlite3    # generated Control Tower checkpoint state
```

The author owns the Workspace configuration, Workflow directories, role files, support code, and application/test data. Control Tower creates and updates `.control_tower/state.sqlite3` as generated checkpoint state; do not edit it to change application state. The example's `data/` and its roles are separate from that database.

## 3. Get the example Workspace

From the repository root, make a disposable copy of the complete simple example. Its UUID-file Workflow uses only a Unix shell:

```sh
example="$(mktemp -d)/simple"
cp -R examples/simple "$example"
workflow="$example/workflows/uuid-file"
printf 'Workspace: %s\nWorkflow: %s\n' "$example" "$workflow"
```

Keep this terminal open for the commands below; its `example` variable is used again. To use the checked-in example in place instead, change to `examples/simple` from the repository root and set `example="$PWD"` and `workflow="$example/workflows/uuid-file"`.

## 4. Prepare and validate the Workflow

Every Workflow has a separate checkpoint database. Prepare it explicitly before use:

```sh
(cd "$example" && control-tower db bootstrap-local --workflow workflows/uuid-file)
(cd "$example" && control-tower db migrate-local --workflow workflows/uuid-file)
(cd "$example" && control-tower db verify-local --workflow workflows/uuid-file)
(cd "$example" && control-tower validate --workflow workflows/uuid-file)
```

Database preparation does not run stages. Ordinary `up`, `down`, and `status` commands do not bootstrap or migrate storage. `validate` checks that the Workspace, Workflow layout, and prepared checkpoint state load; it does not run role files or prove their application-specific behavior.

## 5. Authoring model: make stages meaningful

A stage should represent one coherent mutation that is useful to run, inspect, verify, and, where appropriate, reverse as a unit. Split stages at meaningful state boundaries, not at implementation-command boundaries. A stage may run several commands to create one fixture state; avoid making a stage for each shell command. The UUID-file example demonstrates a useful sequence: create an empty file, write `hello`, then append ` to you`.

The role names are the protocol and must be exact, with no extensions: `up`, `down`, `verify-up`, and `verify-down`. Control Tower uses those names to discover each role and pair directional verification with its mutation. `up` makes the forward change; `down` restores the prior state (or performs an appropriate compensation); `verify-up` checks that the forward change worked; `verify-down` checks that the reversal worked. Verifiers should observe state independently and exit 0 only when it is correct. A verifier is optional. If no compensating action is needed for a direction, an executable `down` that exits 0 without changing anything is an explicit no-op; it allows backward traversal to pass that stage.

Each role can be any directly executable file supported by your system. Its shebang selects the interpreter or runtime, so a role may be a shell script, Python file, compiled executable, or another executable format. Provide the shebang and execute permission, and install any runtime/dependencies yourself. See the [executable contract](../reference/stage-executables.md) for discovery rules and environment details.

## 6. Choose how to operate

The UI and CLI operate the same prepared Workflow. Choose either branch.

### UI branch

From the Workspace root, start the UI and open the local URL it prints:

```sh
(cd "$example" && control-tower ui)
```

Select `uuid-file`, inspect its stages, then use the movement controls to run to stage 3. Keep the UI host running. For the shell inspection below, open a second terminal and set `workflow` to the Workflow path printed in step 3. The UI lists Workflows found at startup; restart the UI host after adding or removing a Workflow. Selecting the current Workflow re-reads its status and stages, but a structural stage edit during a stored run is not reconciled as a generic reload. Finish the run before changing stage structure.

### CLI branch

From the Workspace root, inspect status and run through stage 3:

```sh
(cd "$example" && control-tower status --workflow workflows/uuid-file)
(cd "$example" && control-tower up --workflow workflows/uuid-file --stage 3)
(cd "$example" && control-tower status --workflow workflows/uuid-file)
```

Status should show stage 3 completed. A target stage means “walk through all needed stages up to this number,” not “run only that stage.”

## 7. Inspect the result and return to baseline

Both surfaces use the same ordered movement behavior. The UUID-file stages progressively take the data file from absent, to empty, to `hello`, to `hello to you`. Inspect the fixture from the shell after either branch:

The file should contain `hello to you`:

```sh
cat "$workflow"/data/*
printf '\n'
```

Then return to baseline using the same surface:

- In the UI, choose the backward movement action repeatedly until the displayed position is baseline 0 (the UI offers the immediate reverse transition).
- In the CLI, run these commands from the Workspace root:

  ```sh
  (cd "$example" && control-tower down --workflow workflows/uuid-file --stage 0)
  (cd "$example" && control-tower status --workflow workflows/uuid-file)
  ```

Then inspect the data directory from the shell, whichever surface you used:

```sh
ls -A "$workflow/data"
```

The data directory is empty and the UI/CLI status reports baseline with no active UUID. The prepared database remains, so you can run the Workflow again. A verifier failure leaves a transition pending; inspect the resulting state before retrying the check or backing out. The [navigation guide](verification-and-navigation.md) explains retry and recovery behavior.

## 8. Keep working safely

Role scripts run with your permissions and may change real systems. Control Tower records workflow progress; it cannot guarantee that `down` undoes `up`, sandbox scripts, or reconcile outside effects after a crash. Finish or explicitly recover a run before structural stage edits. The UI's Workflow inventory is fixed at startup, while selecting a Workflow re-reads its status and stage definitions; neither UI restart nor refresh reconciles structural edits with stored run state.

For deeper authoring guidance see [creating a Workflow](creating-a-workflow.md); for UI details, [browser workbench](../reference/browser-workbench.md); for commands, [CLI reference](../reference/cli.md). These links are optional depth; the steps above form the complete first-use path.
