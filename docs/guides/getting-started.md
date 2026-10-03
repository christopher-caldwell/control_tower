---
id: CT-GUIDE-SETUP
title: Build and run Control Tower
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-02'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../Cargo.toml
- ../../crates/cli/Cargo.toml
- ../../crates/database/Cargo.toml
- ../../justfile
- ../decisions/0004-session-storage-port-and-adapters.md
- ../research/documentation-validation.md
---

# Build and run Control Tower

Build both local executables, prepare a disposable workspace, and run the supplied example. Commands below assume a Unix shell and the repository root unless stated otherwise.

## Prerequisites

Install Git and [Rust with Cargo through rustup](https://rust-lang.org/tools/install/). Use a current stable toolchain. You also need a working native C compiler/linker: the `rusqlite` dependency builds bundled SQLite. No PostgreSQL server, SQLite service, SQLite command-line program, Node, Docker, or account is needed for the supplied shell example.

The workspace declares Rust **1.85** as its minimum. The locked workspace test suite passed on Rust 1.85.0 and 1.94.0 on macOS 26.6.2 arm64; see the [macOS delivery validation](../research/2026-10-03-macos-ui-delivery.md). These instructions use Unix tools (`sh`, `mktemp`, `cp`, `chmod`, `cat`). The browser UI is supported on macOS only; native Windows or Linux UI support is not included. The [documentation validation](../research/documentation-validation.md) retains the distinct Linux script-only check and its limits.

Check your tools:

```sh
git --version
rustc --version
cargo --version
cc --version
```

`just` is optional. The primary instructions use the built executables directly.

## Get the source and build

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
cargo build --locked --workspace
```

An existing checkout only needs the build command from its repository root. With Cargo's default target directory, this builds:

| Executable | Purpose |
| --- | --- |
| `target/debug/control-tower` | User commands: `up`, `down`, `status`. |
| `target/debug/control-tower-db` | Explicit local database setup and verification. |

For browser UI preparation and launch requirements, see [Starting the UI](../reference/browser-workbench.md#starting-the-ui). The packaged interface does not require Node; Node is needed only to rebuild its React assets.

`--locked` uses the committed dependency resolution. The initial build needs access to the Rust dependency registry unless those dependencies are already cached. The workbench itself needs no network service; your own scripts may use the network.

```sh
./target/debug/control-tower --help
```

If you configure a different Cargo target directory, adjust the binary paths. See [CLI invocation forms](../reference/cli.md#invocation-forms) for Cargo and installed-binary alternatives.

## Copy the example and select a workspace

Copy the complete simple example so its workflows and dependency metadata stay together, then select the shell-only UUID workspace. It needs no Python or Node setup:

```sh
example="$(mktemp -d)/simple"
cp -R examples/simple "$example"
workspace="$example/workspaces/uuid-file"
printf 'Example workspace: %s\n' "$workspace"
```

Keep the printed path. `$workspace` is a variable in this terminal, not a Control Tower setting. In a later terminal, assign it to that same path before using the examples again. Shell variables do not survive closing a terminal; SQLite state does.

The supplied stage scripts are committed with executable permissions. The [workspace guide](creating-a-workspace.md) explains permissions for scripts you create yourself.

## Set up that workspace's database

The workspace directory must already exist. Run all three operations:

```sh
./target/debug/control-tower-db bootstrap-local "$workspace"
./target/debug/control-tower-db migrate-local "$workspace"
./target/debug/control-tower-db verify-local "$workspace"
```

Bootstrap provisions `.control_tower/state.sqlite3`; migrate applies its schema; verify checks the supported schema version/history. These operations do not run stage scripts. Ordinary `up`, `down`, and `status` open an already prepared database; they do not bootstrap or migrate it.

Set up each new workspace separately. Do not repeat bootstrap as a way to reset a run: it does not clear your saved position. [Database operations](../reference/cli.md#database-operations) explains rerunning setup and adopting the earlier v0 schema.

### Optional `just` shortcuts

If `just` is already installed, the repository provides equivalent commands:

```sh
just db-bootstrap-local "$workspace"
just db-migrate-local "$workspace"
just db-verify-local "$workspace"
```

Run them from the repository root. They invoke the same Database-owned operations through Cargo; they are an alternative to the three binary calls above, not additional setup.

## Run and inspect

```sh
./target/debug/control-tower status --workspace "$workspace"
./target/debug/control-tower up --workspace "$workspace" --stage 3
./target/debug/control-tower status --workspace "$workspace"
cat "$workspace"/data/*
printf '\n'
```

The final status should identify stage 3, show one UUID, and show no pending verification. The file should contain `hello to you`. Read the [example walkthrough](../../examples/simple/workspaces/uuid-file/README.md) to stop and inspect after each stage rather than running all three at once.

```sh
./target/debug/control-tower down --workspace "$workspace" --stage 0
./target/debug/control-tower status --workspace "$workspace"
```

All three down operations and their checks run in reverse order. The fixture file disappears, and status reports baseline with `UUID: not created`. The prepared SQLite database remains; another `up` begins a new run.

## Install the commands on your PATH

This is optional. From the repository root:

```sh
cargo install --locked --path crates/cli --bin control-tower
cargo install --locked --path crates/database --bin control-tower-db
```

Ensure Cargo's install directory is on your `PATH` (normally `~/.cargo/bin`, as described in the [Rust installation notes](https://rust-lang.org/tools/install/)). Install **both** commands: the workbench does not initialize its own database.

You can then use an absolute workspace path from any directory:

```sh
control-tower status --workspace "$workspace"
```

`just` recipes and `cargo run` still require the checkout; the installed executables do not require your current directory to be the repository root.

## Updating and checking the checkout

After updating the source, rebuild the workspace. Run the explicit migration and verification commands for the workspace before using the new binary; do not expect ordinary startup to upgrade it. Installing from a checkout copies a binary, so editing or rebuilding that checkout does not update an already installed command automatically.

To exercise the committed implementation tests:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

The CLI integration suite copies the example and uses real child processes and SQLite. The [run-semantics record](../research/run-semantics-validation.md) contains the earlier executed evidence; the [documentation validation](../research/documentation-validation.md) states what was independently checked during this docs pass.

Continue with [creating a workspace](creating-a-workspace.md), or use [troubleshooting](troubleshooting.md) when a setup command fails.
