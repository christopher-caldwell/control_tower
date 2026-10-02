# PyCapsule example family

These workspaces intentionally share reusable typed Python capabilities. Each
workspace owns its stages and explicit handoffs; the family owns the caller
Python/Node projects and the reusable [example_tool](tools/example_tool/README.md).

> `tools/` is not part of Control Tower's required workspace structure. It is an application-level pattern shown here because multiple workspaces intentionally reuse the same PyCapsule-backed capabilities.

| Workspace | Stages | Lesson |
| --- | --- | --- |
| [tool-only](workspaces/tool-only/README.md) | 3 | Explicit input → typed API → PyCapsule child → explicit output inspection. |
| [node-and-tool](workspaces/node-and-tool/README.md) | 3 | Node input and inspection around an encapsulated Python capability. |
| [full-stack](workspaces/full-stack/README.md) | 7 | Shared tool, ordinary Python, Node, PostgreSQL, native psql, and shell in one investigation. |

## Setup and run

From the Control Tower repository root, install a stable Rust toolchain/linker,
Unix tools, Git, and uv. The examples select Python 3.12. Node/npm are needed for
`node-and-tool` and `full-stack`; PostgreSQL is needed only for `full-stack`.
Your GitHub SSH identity must have read access to the private
`christopher-caldwell/py_capsule` repository to resolve the pinned dependency.

```sh
cargo build --locked --workspace
export PATH="$PWD/target/debug:$PATH"
# Python-only setup:
uv sync --locked --project examples/py_capsule
uv sync --locked --project examples/py_capsule/tools/example_tool
# For Node workflows, also install the family-owned packages:
npm ci --prefix examples/py_capsule --ignore-scripts --no-audit --no-fund
# Or prepare all shared Python and Node dependencies together:
./examples/py_capsule/bootstrap

workspace=examples/py_capsule/workspaces/tool-only
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 3
cat "$workspace/data/tool_result.json"
cat "$workspace/data/runtime_context.json"
control-tower down --workspace "$workspace" --stage 0
```

These commands write ignored output in the authored workspace. For a disposable
copy, copy this family directory including `pyproject.toml`, `uv.lock`,
`.python-version`, Node metadata when needed, `tools/example_tool`, and the chosen
`workspaces/` directory. Exclude existing `.venv`, `node_modules`, `data`, and
`.control_tower` directories, and run dependency setup at the copied family root.
Copying only a PyCapsule workspace omits the shared capability it demonstrates.

## Dependency and execution boundaries

Python roles use `#!/usr/bin/env -S uv run python`. Control Tower invokes them
from their stage directory; uv finds this family's Python project. Stages call:

```python
from example_tool import normalize_record
```

The typed API owns the capsule invocation and runtime injection. The tool owns its
capsule manifest/body and a separate child Python project. The caller environment
and PyCapsule child environment remain distinct. Workspaces persist tool values and
runtime exports as explicit files; Control Tower does not interpret those files.

The shared Python environment includes `python-dateutil` for ordinary timestamp
processing and `psycopg[binary]` for database stages. Node finds family-owned `dayjs`
through normal module resolution. Native shell/psql stages run independently of
Python metadata. Ordinary dependency ownership and nearest-project isolation are
shown in the [simple family](../simple/README.md), without tool encapsulation.

The PyCapsule Git revision remains pinned to
`25edcfe51373cc2ebf0593ae5a033323f71b19e3` in both caller and tool projects because
uv source overrides are not inherited from dependencies. Replacing those source
overrides with a validated released `py-capsule` dependency and regenerating the
two locks will not change stage imports. Publishing PyCapsule is separate work.
The editable example tool keeps its capsule assets in this source tree.

## Recovery and validation

Each stage reverses only its own effects. Tool result and runtime export are
separate writes, so inspect partial local output before retrying a failed tool
stage. This deterministic example can be repeated; it does not establish safe
replay for an arbitrary external API. A fresh tool call starts a fresh simulated
runtime; its previous export is output, not implicit input. Diagnostics remain
under `~/.py_capsule/normalize_record/runs/` after workspace reversal.

`down` reverses accepted transitions. Partial effects of a failed mutation belong
to the workflow. See the [full-stack recovery procedure](workspaces/full-stack/README.md#failed-stage-003-retry-or-abandon)
and maintained [navigation guidance](../../docs/guides/verification-and-navigation.md#a-mutation-failure-is-not-a-verifier-failure).

```sh
./examples/test --family py_capsule
./examples/test --family py_capsule --postgres
```

The suite checks real child execution, typed API usage, shared environments,
explicit handoffs, verifier retries, locks, and native runtime independence.
Database tests use private disposable clusters. When dependency metadata changes,
update `uv.lock` at the family and affected tool project, or `package-lock.json`
for Node, then rerun setup and tests. Setup enforces locks; ordinary stage shebangs
remain plain `uv run python`, with lock stability checked separately.

These capabilities originated in `christopher-caldwell/control-tower-py-capsule-demo`,
branch `feat/workspace-pattern-gallery`, commit
`2c8095e8fab163b710166d89aedd6f57241b99a9`. Control Tower now owns this gallery's evolution.
Return to the [gallery](../README.md).
