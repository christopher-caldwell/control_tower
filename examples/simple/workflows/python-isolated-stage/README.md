# A Python stage with its own dependency

Most stages use example-owned `python-dateutil`. Stage 003 performs real JSON
Schema validation and owns a closer `pyproject.toml` / `uv.lock` with `jsonschema`.
That dependency is absent from the parent environment, so isolation has a concrete
reason. The isolated stage consumes files and does not need to import parent code.

## Prerequisites and progression

Use a Unix shell, the built `control-tower` CLI, and uv and Python 3.12.
Follow [example setup](../../README.md#setup-and-run), then run these commands from the copied example root.

```text
001 Python seed (example dateutil) -> data/record.json
002 Python normalize (example dateutil) -> data/normalized.json and data/inspection.json
003 Python schema validation (stage jsonschema) -> data/validated.json
```

Application data moves through the named files under `data/`; stdout is feedback.
Every stage has forward/reverse verification. Verifiers observe rather than repair
outputs. Dependencies belong to the containing example; each workflow owns its stage outputs.

## Run the workflow

```sh
workflow=workflows/python-isolated-stage
control-tower db bootstrap-local "$workflow"
control-tower db migrate-local "$workflow"
control-tower db verify-local "$workflow"
# Use --stage 1, then each successive stage to inspect intermediate files.
control-tower up --workflow "$workflow" --stage 3
cat "$workflow/data/normalized.json"
cat "$workflow/data/validated.json"
control-tower status --workflow "$workflow"
control-tower down --workflow "$workflow" --stage 0
control-tower status --workflow "$workflow"
```

Expected values are record ID `123`, label `EXAMPLE RECORD`, and date `2026-01-02`
in the final JSON output. Reversing to baseline removes stage-owned outputs and
clears the active UUID; the checkpoint database and installed environments/build
caches remain. Each `down` removes only its named output files. Keep unrelated data
away from these paths: reversal does not restore overwritten files.

If a mutation fails after writing a file, inspect its partial output before
retrying. These deterministic transformations can overwrite their own outputs on
retry. After a verifier failure, repeating the same move retries only verification.
For the global rules, see [navigation and verification](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/verification-and-navigation.md)
and the [executable contract](https://github.com/christopher-caldwell/control_tower/blob/main/docs/reference/stage-executables.md).
Return to the [example](../../README.md).
