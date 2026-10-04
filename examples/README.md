# Control Tower examples

An **example** is the complete portable Workspace you copy. Its **workflows**
are the runnable scenarios inside that Workspace. Control Tower executes
ordinary role files directly; each role selects its runtime through its shebang.

| Example to copy | What it teaches | Where to start |
| --- | --- | --- |
| [simple](simple/README.md) | Focused workflows, ordinary dependencies, stage isolation, and database ownership. | [uuid-file](simple/workflows/uuid-file/README.md). |
| [multi_language](multi_language/README.md) | Ordinary runtimes composed through explicit files. | [shell-python-node](multi_language/workflows/shell-python-node/README.md). |
| [py_capsule](py_capsule/README.md) | Shared encapsulated tools and richer workflows. | [tool-only](py_capsule/workflows/tool-only/README.md), then full-stack. |

```text
examples/
  simple/                     a Workspace: copy this complete example
    workflows/                focused scenarios
  multi_language/             a Workspace: copy this complete example
    workflows/                ordinary runtime compositions
  py_capsule/                 a Workspace: copy this complete example
    tools/                    shared encapsulated capabilities
    workflows/                scenarios consuming those capabilities
```

For example, `cp -R examples/simple ~/somewhere/simple` retains everything
repository-authored needed by its workflows. Install the documented external CLIs,
runtimes, and services, then follow setup at the copied example root. Workflows
may use their containing Workspace's configuration and helpers; their directories
are not the portability boundary. Only PyCapsule demonstrates a reusable `tools/`
layer. Control Tower gives that directory no special meaning.

The [root quickstart](../README.md#try-the-three-stage-example) uses shell and built
Control Tower binaries. See [workflow authoring](../docs/guides/creating-a-workflow.md)
and the [executable contract](../docs/reference/stage-executables.md) for global semantics.

## Optional integration tests

Build this checkout with `cargo build --locked --workspace`, then run:

```sh
./examples/test
./examples/test --family simple
./examples/test --family multi_language
./examples/test --family py_capsule
./examples/test --postgres
# Options can be combined; repeat --family to select several examples.
./examples/test --family simple --postgres
```

The test harness owns its Python dependencies under `tests/`; stages never use
them. Tests run actual Control Tower binaries against disposable copies, including
paths containing spaces and quotes. Complete example copies live in pytest temporary
directories outside the repository; JUnit reports live in ignored `test-results/`. Core `cargo test --locked --workspace` does not run these suites.

Missing optional runtime/toolchain executables produce explicit skips. Available
runtimes with failed setup or execution produce test failures. PyCapsule installs
the pinned `capsule-runner==0.0.1` release from PyPI without private Git access.
PostgreSQL tests are explicitly opt-in; `--postgres` requires `initdb`, `pg_ctl`, and `psql`, starts private disposable
clusters without TCP listeners, and never uses your application DSN.
