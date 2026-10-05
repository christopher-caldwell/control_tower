# Multi-language example

**Control Tower does not have a runtime. Each stage chooses its runtime.**
Copy this complete example to experiment with ordinary languages composed through
explicit files. Runtime boundaries are visible in the executable stage files.

| Workflow | Prerequisites beyond Control Tower and a Unix shell | Progression |
| --- | --- | --- |
| [shell-python-node](workflows/shell-python-node/README.md) | uv / Python 3.12, Node / npm | Shell input → Python normalization → Node inspection. |
| [go-rust](workflows/go-rust/README.md) | Go 1.23+, Rust/Cargo and a linker | Go input → Rust transformation. |

## Setup and run

Build or install `control-tower` and put it on PATH.
See the repository's [CLI setup guide](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/getting-started.md).
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
workflow. The first forward traversal compiles them with the ordinary toolchains.
This example has no reusable `tools/` layer.

```sh
workflow=workflows/shell-python-node
control-tower db bootstrap-local --workflow "$workflow"
control-tower db migrate-local --workflow "$workflow"
control-tower db verify-local --workflow "$workflow"
control-tower up --workflow "$workflow" --stage 3
cat "$workflow/data/node_inspection.json"
control-tower down --workflow "$workflow" --stage 0
```

Use `workflow=workflows/go-rust` and target stage 2 for the other workflow.
Workflow READMEs describe intermediate files and reversal.

From the Control Tower repository root, run `./examples/test --family multi_language`.
Unavailable optional toolchains produce skips; setup, compilation, and execution
errors with available tools produce failures. Return to the [gallery](https://github.com/christopher-caldwell/control_tower/blob/main/examples/README.md).
