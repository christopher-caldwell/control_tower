# Control Tower

A local CLI workbench for stepping through your own executable actions: set up a test fixture, verify it, change your application, then move backward and try again.

You write `up`, `down`, and optional `verify-up` / `verify-down` files. Control Tower runs them in order and remembers your position between commands. No server, account, or built-in HTTP/database action language.

## Try the three-stage example

You need Git, a current stable Rust toolchain with Cargo, a C compiler/linker, and a Unix shell. The recorded full CLI tests ran on macOS; native Windows is not verified. See [setup and toolchain notes](docs/guides/getting-started.md#prerequisites). **Neither `just` nor a separate SQLite installation is required for this path.**

Run these commands in one terminal. Skip the clone when you already have the repository, and start from its root.

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
cargo build --locked --workspace

workspace="$(mktemp -d)"
cp -R examples/uuid-file/. "$workspace/"
printf 'Example workspace: %s\n' "$workspace"

./target/debug/control-tower-db bootstrap-local "$workspace"
./target/debug/control-tower-db migrate-local "$workspace"
./target/debug/control-tower-db verify-local "$workspace"

./target/debug/control-tower up --workspace "$workspace" --stage 3
./target/debug/control-tower status --workspace "$workspace"
cat "$workspace"/data/*
printf '\n'
```

The file should contain **`hello to you`**, and status should report completed stage 3. One UUID-named file progresses through:

| Completed stage | Example file |
| --- | --- |
| 0 | Absent |
| 1 | Empty |
| 2 | `hello` |
| 3 | `hello to you` |

Back out the example:

```sh
./target/debug/control-tower down --workspace "$workspace" --stage 0
./target/debug/control-tower status --workspace "$workspace"
```

The example file is now absent; status reports baseline 0 and no UUID. The temporary workspace and its SQLite file remain available for another run. Each command is a separate process. Database setup is explicit and does not run during `up`, `down`, or `status`.

## Use it in your work

[Walk through the example one stage at a time](examples/uuid-file/README.md), then [create your own workspace](docs/guides/creating-a-workspace.md). A failed verifier leaves the stage unfinished: repeat the direction to retry the check, or request the opposite direction to back it out. [Navigation and verification](docs/guides/verification-and-navigation.md) explains the loop.

For a database-generated record ID carried between stages in an author-owned file, try the optional [two-stage generated-ID example](examples/generated-id/README.md). That example additionally requires Python 3's standard-library SQLite module.

Scripts run with your permissions and can change real systems. Control Tower does not guarantee that `down` undoes `up`, provide a sandbox, or reconcile external effects after a crash.

[Setup and installation](docs/guides/getting-started.md) · [CLI reference](docs/reference/cli.md) · [Executable and environment contract](docs/reference/stage-executables.md) · [Troubleshooting](docs/guides/troubleshooting.md) · [All documentation](docs/README.md)
