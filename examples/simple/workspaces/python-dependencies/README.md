# Ordinary Python dependencies

The simple example owns `pyproject.toml` and `uv.lock`. Both stages discover its
Python environment through plain `uv run python`; `python-dateutil`
parses the input timestamp during seed and normalization.

## Prerequisites and progression

Use a Unix shell, built Control Tower binaries, and uv and Python 3.12.
Follow [example setup](../../README.md#setup-and-run), then run these commands from the copied example root.

```text
001 Python seed (dateutil) -> data/record.json
002 Python normalize (dateutil) -> data/normalized.json and data/inspection.json
```

Application data moves through the named files under `data/`; stdout is feedback.
Every stage has forward/reverse verification. Verifiers observe rather than repair
outputs. Dependencies belong to the containing example; each workspace owns its stage outputs.

## Run the workspace

```sh
workspace=workspaces/python-dependencies
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
# Use --stage 1, then each successive stage to inspect intermediate files.
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/normalized.json"
cat "$workspace/data/inspection.json"
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
control-tower status --workspace "$workspace"
```

Expected values are record ID `123`, label `EXAMPLE RECORD`, and date `2026-01-02`
in the final JSON output. Reversing to baseline removes stage-owned outputs and
clears the active UUID; the checkpoint database and installed environments/build
caches remain. Each `down` removes only its named output files. Keep unrelated data
away from these paths: reversal does not restore overwritten files.

If a mutation fails after writing a file, inspect its partial output before
retrying. These deterministic transformations can overwrite their own outputs on
retry. After a verifier failure, repeating the same move retries only verification.
For the global rules, see [navigation and verification](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/docs/guides/verification-and-navigation.md)
and the [executable contract](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/docs/reference/stage-executables.md).
Return to the [example](../../README.md).
