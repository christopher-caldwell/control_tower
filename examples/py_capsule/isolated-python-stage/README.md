# An exceptional isolated inspection stage

Stages 001 and 002 use the shared category default without local Python configuration.
Stage 003 validates the normalized record against a JSON Schema before writing its
inspection. Its closer `pyproject.toml`, `uv.lock`, and `.venv` supply `jsonschema`;
the shared environment lacks that library. The isolated environment excludes
`example-tool` and exchanges explicit files with the preceding stage.

This is the escape hatch when a stage needs separate analysis requirements.
`jsonschema` could reasonably become a shared dependency if validation became common;
an isolated environment is not required just because a stage uses Python.

See [usage and recovery](../README.md). Finish at stage 3 and reverse to stage 0.

## Run it

After [category setup](../README.md#setup-and-run), run from the repository root:

```sh
workspace=examples/py_capsule/isolated-python-stage
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 3
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
```

Inspect `data/` between stages; the category walkthrough shows one-stage movement.
