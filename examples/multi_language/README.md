# Multi-language example

**Control Tower does not have a runtime. Each stage chooses its runtime.**
Copy this complete example to experiment with ordinary languages composed through
explicit files. Runtime boundaries are visible in the executable stage files.

| Workspace | Prerequisites beyond Control Tower and a Unix shell | Progression |
| --- | --- | --- |
| [shell-python-node](workspaces/shell-python-node/README.md) | uv / Python 3.12, Node / npm | Shell input → Python normalization → Node inspection. |
| [go-rust](workspaces/go-rust/README.md) | Go 1.23+, Rust/Cargo and a linker | Go input → Rust transformation. |

## Setup and run

Install or build `control-tower` and `control-tower-db` and put both on PATH.
See the repository's [CLI setup guide](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/docs/guides/getting-started.md).
From the Control Tower checkout:

```sh
example="$(mktemp -d /tmp/control-tower-multi-language.XXXXXX)/multi_language"
cp -R examples/multi_language "$example"
cd "$example"
```

Copy the authored tree before running it. When copying an already-used example,
exclude generated `.venv`, `node_modules`, `data`, `.control_tower`, and `target`.
For shell-python-node, install the example-owned dependencies:

```sh
uv sync --locked
npm ci --ignore-scripts --no-audit --no-fund
```

uv discovers example-owned `python-dateutil` from Python stage directories;
Node resolves example-owned `dayjs`. Go/Rust needs neither of these commands.
Its module and Cargo project define that workflow's programs and remain in its
workspace. The first forward traversal compiles them with the ordinary toolchains.
This example has no reusable `tools/` layer.

```sh
workspace=workspaces/shell-python-node
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 3
cat "$workspace/data/node_inspection.json"
control-tower down --workspace "$workspace" --stage 0
```

Use `workspace=workspaces/go-rust` and target stage 2 for the other workflow.
Workspace READMEs describe intermediate files and reversal.

From the Control Tower repository root, run `./examples/test --family multi_language`.
Unavailable optional toolchains produce skips; setup, compilation, and execution
errors with available tools produce failures. Return to the [gallery](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/examples/README.md).
