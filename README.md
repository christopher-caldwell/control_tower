# Control Tower

A local CLI workbench, with an optional loopback browser UI, for stepping through your own executable actions: set up a test fixture, verify it, change your application, then move backward and try again.

You write `up`, `down`, and optional `verify-up` / `verify-down` files. Control Tower runs them in order and remembers your position between commands. No hosted server, account, or built-in HTTP/database action language.

## Browser workbench

Run `control-tower ui` from a Workspace root to print a local URL, then open it in your browser. A **Workspace** has a `control-tower.toml` with a nonempty `[workspace].label` and a required `workflows/` folder; each **Workflow** inside it owns a `stages/` folder and its own checkpoint database. CLI workflow commands require `--workflow` and resolve its path from that Workspace root. The [browser workbench reference](docs/reference/browser-workbench.md) explains workflow inventory and use.

## UI development

For UI development, run `pnpm --dir ui install --frozen-lockfile`, then
`just dev-ui` (or `pnpm --dir ui dev`). Open the printed Vite URL. This starts
hot reload and the real Rust API with a prepared, reusable sample workspace.
See the [UI development guide](ui/README.md#develop-against-a-real-workspace)
for custom workspaces and sample reset instructions.

## Agent guidance

Coding agents can run `control-tower guide` for the embedded guide index and then request one scoped guide action. The shipped dispatcher Skill is [`skills/control-tower/SKILL.md`](skills/control-tower/SKILL.md). From the Workspace root, `control-tower validate --workflow workflows/NAME` checks that the selected layout and prepared checkpoint state load without running stage roles. The Workspace-root `.env` supplies default stage-executable values; shell values take precedence, followed by authoritative Control Tower variables.

Run `just install-cli` to install the CLI and copy skills to `~/.agents/skills`. Use `just install-cli claude` to copy them to `~/.claude/skills` instead.

## Try the three-stage example

You need Git, a current stable Rust toolchain with Cargo, a C compiler/linker, and a Unix shell. The recorded full CLI tests ran on macOS; native Windows is not verified. See [setup and toolchain notes](docs/guides/getting-started.md#prerequisites). **Neither `just` nor a separate SQLite installation is required for this path.**

Run these commands in one terminal. Skip the clone when you already have the repository, and start from its root.

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
cargo build --locked --workspace

example="$(mktemp -d)/simple"
cp -R examples/simple "$example"
workflow="$example/workflows/uuid-file"
printf 'Example workflow: %s\n' "$workflow"

binary="$PWD/target/debug/control-tower"
(cd "$example" && "$binary" db bootstrap-local --workflow workflows/uuid-file)
(cd "$example" && "$binary" db migrate-local --workflow workflows/uuid-file)
(cd "$example" && "$binary" db verify-local --workflow workflows/uuid-file)

(cd "$example" && "$binary" up --workflow workflows/uuid-file --stage 3)
(cd "$example" && "$binary" status --workflow workflows/uuid-file)
cat "$workflow"/data/*
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
(cd "$example" && "$binary" down --workflow workflows/uuid-file --stage 0)
(cd "$example" && "$binary" status --workflow workflows/uuid-file)
```

The example file is now absent; status reports baseline 0 and no UUID. The temporary workflow and its SQLite file remain available for another run. Each command is a separate process. Database setup is explicit and does not run during `up`, `down`, or `status`.

## Use it in your work

[Walk through the example one stage at a time](examples/simple/workflows/uuid-file/README.md), then [create your own workflow](docs/guides/creating-a-workflow.md). A failed verifier leaves the stage unfinished: repeat the direction to retry the check, or request the opposite direction to back it out. [Navigation and verification](docs/guides/verification-and-navigation.md) explains the loop.

For a database-generated record ID carried between stages in an author-owned file, try the optional [two-stage generated-ID example](examples/simple/workflows/generated-id/README.md). That example additionally requires Python 3's standard-library SQLite module.

The [example gallery](examples/README.md) also includes ordinary Python/Node dependencies, multi-language composition, PostgreSQL recovery, and shared PyCapsule tools. Copy a complete example directory; its README explains setup and workflow selection.

Scripts run with your permissions and can change real systems. Control Tower does not guarantee that `down` undoes `up`, provide a sandbox, or reconcile external effects after a crash.

The [core-v0 completion record](docs/research/2026-10-01-core-v0-completion.md) establishes the current navigation contract as ready for repeated local owner use, with executed evidence and platform/toolchain limits. Real development use should drive the next changes.

[Setup and installation](docs/guides/getting-started.md) · [CLI reference](docs/reference/cli.md) · [Executable and environment contract](docs/reference/stage-executables.md) · [Troubleshooting](docs/guides/troubleshooting.md) · [All documentation](docs/README.md)
