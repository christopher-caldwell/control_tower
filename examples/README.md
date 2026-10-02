# Control Tower examples

An **example family** groups related scenarios. A **workspace** is the runnable
scenario you copy or run. Control Tower executes the workspace's ordinary role
files directly; each role selects its own runtime through its shebang.

| Family | What it teaches | Where to start |
| --- | --- | --- |
| [simple](simple/README.md) | Focused Control Tower usage, ordinary dependencies, isolation, and database ownership. | [uuid-file](simple/workspaces/uuid-file/README.md), the smallest workflow. |
| [multi_language](multi_language/README.md) | Ordinary runtimes composed through explicit files: individual stages own their runtimes. | [shell-python-node](multi_language/workspaces/shell-python-node/README.md). |
| [py_capsule](py_capsule/README.md) | Several workspaces intentionally reuse typed, encapsulated Python tools. | [tool-only](py_capsule/workspaces/tool-only/README.md), then the full-stack investigation. |

```text
examples/
  simple/workspaces/           independently copyable scenarios
  multi_language/workspaces/   independently copyable runtime compositions
  py_capsule/
    tools/                    intentionally shared reusable capabilities
    workspaces/               scenarios consuming those capabilities
```

Simple and multi-language dependencies belong to individual workspaces. Copying
one workspace supplies its scenario-specific files; install its external tools
separately. PyCapsule workspaces intentionally depend on their shared family
projects and sibling tool. Only that family introduces a reusable `tools/` layer.

The [root quickstart](../README.md#try-the-three-stage-example) uses shell and built
Control Tower binaries. See [workspace authoring](../docs/guides/creating-a-workspace.md)
and the [executable contract](../docs/reference/stage-executables.md) for global semantics.

## Optional integration tests

Build this checkout with `cargo build --locked --workspace`, then run:

```sh
./examples/test
./examples/test --family simple
./examples/test --family multi_language
./examples/test --family py_capsule
./examples/test --postgres
# Options can be combined; repeat --family to select several families.
./examples/test --family simple --postgres
```

The test harness owns its Python dependencies under `tests/`; stages never use
them. Tests run actual Control Tower binaries against disposable copies, including
paths containing spaces and quotes. Reports and copied workspaces live in ignored
`test-results/`. Core `cargo test --locked --workspace` does not run these suites.

Missing optional runtime/toolchain executables produce explicit skips. Available
runtimes with failed setup or execution produce test failures. PyCapsule requires
read access to its pinned private Git dependency. PostgreSQL tests are explicitly
opt-in; `--postgres` requires `initdb`, `pg_ctl`, and `psql`, starts private disposable
clusters without TCP listeners, and never uses your application DSN.
