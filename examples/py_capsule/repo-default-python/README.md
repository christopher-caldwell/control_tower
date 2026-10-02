# Default category Python environment

The three stages contain executable roles only. Their plain `uv run python`
shebangs discover the shared category project, which installs `example_tool` as a typed
editable import. No workspace/stage project, lock, or Python setting is needed.

Stage 001 also imports `dateutil.parser.isoparse` from the small `python-dateutil`
dependency declared in the category. It parses the fixture's ISO timestamp and writes
the normalized timestamp into `data/record.json` beside the label and id.

The reusable tool owns PyCapsule's child environment; stages own all explicit
scenario reads and writes. See [usage and recovery](../README.md). Finish at stage
3 and reverse to stage 0.

The scenario handoff is visible in the roles:

```text
001 seed: parse a timestamp and write data/record.json
002 call tool: read record.json, normalize through PyCapsule,
    write data/tool_result.json and data/runtime_context.json separately
003 inspect: read those outputs and write data/inspection.json
```

Verifiers read these files without calling the tool or repairing data. Reversal
removes only the corresponding stage's outputs.

## Run it

After [category setup](../README.md#setup-and-run), run from the repository root:

```sh
workspace=examples/py_capsule/repo-default-python
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 3
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
```

Inspect `data/` between stages; the category walkthrough shows one-stage movement.
