# Simple example

Copy this complete example to experiment with focused Control Tower workflows.
The workflows share ordinary Python and Node dependencies; each owns its stages,
application data, and Control Tower checkpoint database.

| Workflow | Additional prerequisites | Lesson |
| --- | --- | --- |
| [uuid-file](workflows/uuid-file/README.md) | Unix shell | One runner UUID, verification, and reversal. |
| [generated-id](workflows/generated-id/README.md) | Python 3 with standard-library SQLite | Explicit handoff of an application-generated identifier. |
| [python-dependencies](workflows/python-dependencies/README.md) | uv / Python 3.12 | Ordinary example-owned Python dependencies. |
| [node-dependencies](workflows/node-dependencies/README.md) | Node / npm | Ordinary example-owned Node dependencies. |
| [python-isolated-stage](workflows/python-isolated-stage/README.md) | uv / Python 3.12 | A closer stage project supplies a different dependency. |
| [postgres](workflows/postgres/README.md) | uv / Python 3.12 and PostgreSQL | Run-owned external effects, reversal, retry, and abandonment. |

## Setup and run

Install `control-tower` and put it on PATH, then follow the repository's
[canonical getting-started walkthrough](../../docs/guides/getting-started.md).
You need a Unix shell; install additional tools only for the scenarios you choose.

From the Control Tower checkout, copy the complete example, then enter the copy:

```sh
example="$(mktemp -d /tmp/control-tower-simple.XXXXXX)/simple"
cp -R examples/simple "$example"
cd "$example"
```

Copy the authored tree before running it. When copying an already-used example,
exclude generated `.venv`, `node_modules`, `data`, `.control_tower`, and caches.

For the Python dependency, isolated-stage, or PostgreSQL workflows:

```sh
uv sync --locked
```

This installs ordinary `python-dateutil` and `psycopg[binary]` in the example's
`.venv`. Stage working directories naturally discover this project. For the
isolated-stage workflow, also prepare its closer project:

```sh
uv sync --locked --project workflows/python-isolated-stage/stages/003-inspect
```

That stage uses `jsonschema`, which is absent from the example environment.
For the Node workflow:

```sh
npm ci --ignore-scripts --no-audit --no-fund
```

Node roles resolve example-owned `dayjs` through normal ancestor lookup.
The UUID workflow needs neither setup command; generated-ID uses only Python's
standard library. PostgreSQL server/schema setup is documented in its workflow.

Select a workflow from this example root:

```sh
workflow=workflows/uuid-file
control-tower db bootstrap-local --workflow "$workflow"
control-tower db migrate-local --workflow "$workflow"
control-tower db verify-local --workflow "$workflow"
control-tower up --workflow "$workflow" --stage 3
cat "$workflow"/data/*
control-tower down --workflow "$workflow" --stage 0
```

Each workflow README supplies its stage targets, artifacts, and recovery steps.
Dependencies are installed once per example. Schema and generated-ID support code
stay with their scenarios; only the exceptional Python stage has its own project.

From the Control Tower repository root, run `./examples/test --family simple`;
add `--postgres` for private disposable database tests. Return to the
[gallery](https://github.com/christopher-caldwell/control_tower/blob/main/examples/README.md), or see the [PyCapsule example](https://github.com/christopher-caldwell/control_tower/blob/main/examples/py_capsule/README.md)
for shared encapsulated tools.
