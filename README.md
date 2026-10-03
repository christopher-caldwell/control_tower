# Control Tower

A local CLI workbench, with an optional macOS browser UI, for stepping through your own executable actions: set up a test fixture, verify it, change your application, then move backward and try again.

You write `up`, `down`, and optional `verify-up` / `verify-down` files. Control Tower runs them in order and remembers your position between commands. No hosted server, account, or built-in HTTP/database action language.

## Browser workbench (macOS)

The browser UI uses the same Rust executable and a prebuilt React interface. It is a read-only stage inspector in this first slice; it does not execute scripts. Build and launch it from a copied Project directory:

```sh
cargo build --locked --release --workspace
repo="$(pwd)"
project="$(mktemp -d)/simple"
cp -R "$repo/examples/simple" "$project"
workspace="$project/workspaces/uuid-file"
"$repo/target/release/control-tower-db" bootstrap-local "$workspace"
"$repo/target/release/control-tower-db" migrate-local "$workspace"
"$repo/target/release/control-tower-db" verify-local "$workspace"
(cd "$project" && "$repo/target/release/control-tower" ui)
```

The Project launch directory supplies workspace context. Workspaces switch inside the UI; project switching is not included. Node is only needed when rebuilding frontend source, not to run the packaged interface. See the [browser workbench reference](docs/reference/browser-workbench.md) for setup, security, scope, and lifecycle details.

## Try the three-stage example

You need Git, a current stable Rust toolchain with Cargo, a C compiler/linker, and a Unix shell. The recorded full CLI tests ran on macOS; native Windows is not verified. See [setup and toolchain notes](docs/guides/getting-started.md#prerequisites). **Neither `just` nor a separate SQLite installation is required for this path.**

Run these commands in one terminal. Skip the clone when you already have the repository, and start from its root.

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
cargo build --locked --workspace

example="$(mktemp -d)/simple"
cp -R examples/simple "$example"
workspace="$example/workspaces/uuid-file"
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

[Walk through the example one stage at a time](examples/simple/workspaces/uuid-file/README.md), then [create your own workspace](docs/guides/creating-a-workspace.md). A failed verifier leaves the stage unfinished: repeat the direction to retry the check, or request the opposite direction to back it out. [Navigation and verification](docs/guides/verification-and-navigation.md) explains the loop.

For a database-generated record ID carried between stages in an author-owned file, try the optional [two-stage generated-ID example](examples/simple/workspaces/generated-id/README.md). That example additionally requires Python 3's standard-library SQLite module.

The [example gallery](examples/README.md) also includes ordinary Python/Node dependencies, multi-language composition, PostgreSQL recovery, and shared PyCapsule tools. Copy a complete example directory; its README explains setup and workspace selection.

Scripts run with your permissions and can change real systems. Control Tower does not guarantee that `down` undoes `up`, provide a sandbox, or reconcile external effects after a crash.

The [core-v0 completion record](docs/research/2026-10-01-core-v0-completion.md) establishes the current navigation contract as ready for repeated local owner use, with executed evidence and platform/toolchain limits. Real development use should drive the next changes.

[Setup and installation](docs/guides/getting-started.md) · [CLI reference](docs/reference/cli.md) · [Executable and environment contract](docs/reference/stage-executables.md) · [Troubleshooting](docs/guides/troubleshooting.md) · [All documentation](docs/README.md)
