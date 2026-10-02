# Control Tower with PyCapsule

These examples compose ordinary executable roles with a typed PyCapsule tool.
Control Tower directly executes a role; the role chooses its runtime. The shared
Python and Node projects belong to this category, not the Control Tower root.

| Workflow | Final stage | Lesson |
| --- | --- | --- |
| [repo-default-python](repo-default-python/README.md) | 3 | Ordinary Python stages import a typed tool without local project metadata. |
| [isolated-python-stage](isolated-python-stage/README.md) | 3 | A closer project supplies JSON Schema validation to one exceptional stage. |
| [polyglot](polyglot/README.md) | 4 | Shell → Python/PyCapsule → Node → shell, with normal Node module resolution. |
| [postgres](postgres/README.md) | 5 | Run-owned database writes, Python/native psql inspection, reversal, and partial-effect recovery. |

## Setup and run

Start from the Control Tower repository root. Install Unix tools, Git, a stable
Rust toolchain/C compiler, uv, and Node/npm. This category selects Python 3.12.
PyCapsule is private: your GitHub SSH identity must have read access to
`christopher-caldwell/py_capsule`. Repository clone access alone is not sufficient.
Setup retrieves a pinned Git dependency through uv; no token belongs in these files.

```sh
cargo build --locked --workspace
./examples/py_capsule/bootstrap
export PATH="$PWD/target/debug:$PATH"

workspace=examples/py_capsule/repo-default-python
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 1
cat "$workspace/data/record.json"
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/tool_result.json"
cat "$workspace/data/runtime_context.json"
control-tower up --workspace "$workspace" --stage 3
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
```

This walkthrough writes ignored runtime files beside the example. Tests instead use
fresh disposable copies. See each workflow README for its workspace and target;
PostgreSQL server/schema setup is explicitly opt-in.

## Shared environment and tool boundary

An ordinary Python role starts with:

```python
#!/usr/bin/env -S uv run python
from example_tool import normalize_record
```

Control Tower runs the executable with its stage working directory. uv walks upward
to this category's `pyproject.toml`: ordinary workspaces/stages need no project,
lock, `--project`, `--locked`, `UV_LOCKED`, or `PYTHONPATH` configuration. The shared
project installs editable `example-tool`, `python-dateutil` for timestamp parsing,
and `psycopg[binary]` for the optional PostgreSQL workflow. Installing that driver
does not start a server.

Use the shared category environment by default. Stage 003 of
`isolated-python-stage` has a closer project for actual `jsonschema` validation;
that dependency is absent from the shared environment. The isolated stage reads
explicit files and does not import the tool.

[example_tool](tools/example_tool/README.md) owns the typed API, PyCapsule invocation,
capsule manifest/body, injected Session/Conversation runtime, and its child Python
project. The child environment is distinct from the caller environment. Workspaces
own sequencing, explicit input reads/output writes, validation, and reversal.
Control Tower neither parses these files nor turns stdout into application state.

Node uses this category's `package.json`, `package-lock.json`, and `node_modules`
through normal module resolution. Setup runs `npm ci`; no role installs packages.
Shell/Node/native psql roles run independently of Python metadata.

## Local validation and maintenance

```sh
./examples/py_capsule/test
# Optional: requires initdb, pg_ctl, and psql; starts private disposable clusters.
./examples/py_capsule/test --postgres
```

The category-local pytest suite runs this checkout's built binaries, uv shebangs,
and the real PyCapsule child. It checks intermediate artifacts, reversal, fresh
runs, verifier-only retries, environment ownership, lock stability, and native
runtime independence. Reports and disposable workspaces live in `test-results/`.
The PostgreSQL tests ignore your application DSN, start clusters with private Unix
sockets/no TCP listener, and test both retry and abandonment after commit/receipt
failure. Ordinary tests start no database server.

When dependency metadata changes, update each affected independently owned lock:

```sh
uv lock --project examples/py_capsule
uv lock --project examples/py_capsule/tools/example_tool
uv lock --project examples/py_capsule/isolated-python-stage/stages/003-inspect
./examples/py_capsule/bootstrap
./examples/py_capsule/test
```

Setup enforces locks. Ordinary roles deliberately use plain `uv run python`, which
may update a stale lock; tests separately verify strict stale-lock rejection.
For editor completion, select `examples/py_capsule/.venv/bin/python`; the isolated
stage has its own interpreter. Extensionless roles may need Python language mode.

## Recovery and provenance

The file workflows write `tool_result.json` and `runtime_context.json` separately;
a write failure may leave only one. Inspect and repair local output before retrying
a failed mutation, which invokes the tool again. These sample normalization/file
writes are deterministic; they do not establish safe replay of a real external API
call. A tool call starts a fresh simulated runtime with empty incoming context and
does not consume its previous export as input.

Each `down` removes its own named outputs and does not restore overwritten files;
keep unrelated files away from those paths. PyCapsule diagnostics remain under
`~/.py_capsule/normalize_record/runs/`, and the Control Tower checkpoint database
remains after reversal.

Each `down` owns only its stage's effects. Deleting local output does not compensate
arbitrary external calls. A successful mutation with a failed verifier can retry
only the check. A failed mutation can leave partial effects without being accepted;
normal `down` traversal does not reverse that unaccepted stage. See the maintained
[navigation guidance](../../docs/guides/verification-and-navigation.md#a-mutation-failure-is-not-a-verifier-failure)
and the concrete [Postgres recovery procedure](postgres/README.md#failed-stage-002-retry-or-abandon).

These direct-composition workflows originated in
`christopher-caldwell/control-tower-py-capsule-demo`, branch
`feat/workspace-pattern-gallery`, commit
`2c8095e8fab163b710166d89aedd6f57241b99a9`. Control Tower now owns their evolution.
PyCapsule uses development Git wiring pinned to
`25edcfe51373cc2ebf0593ae5a033323f71b19e3`, declared in both category and tool sources
because a dependency's uv source overrides are not inherited. The intended later
state is a released `py-capsule` dependency: remove those overrides and regenerate
the two locks when a validated release is available. Publishing PyCapsule is separate
work. The local editable example tool is not presented as a distributable wheel;
its capsule assets and child project remain source-owned.
