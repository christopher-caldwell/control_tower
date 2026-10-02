# Ordinary Python dependencies

A normal Python workspace owns `pyproject.toml` and `uv.lock`. Both stages use the
nearest workspace environment through plain `uv run python`; `python-dateutil`
parses the input timestamp during seed and normalization.

## Prerequisites and progression

Use a Unix shell, built Control Tower binaries, and uv and Python 3.12.
Start the commands below from the Control Tower repository root.

```text
001 Python seed (dateutil) -> data/record.json
002 Python normalize (dateutil) -> data/normalized.json and data/inspection.json
```

Application data moves through the named files under `data/`; stdout is feedback.
Every stage has forward/reverse verification. Verifiers observe rather than repair
outputs. This workspace has no dependency on sibling workspaces or family helpers.

## Run a disposable copy

```sh
cargo build --locked --workspace
export PATH="$PWD/target/debug:$PATH"
workspace=$(mktemp -d /tmp/control-tower-python-dependencies.XXXXXX)
cp -R examples/simple/workspaces/python-dependencies/. "$workspace/"
uv sync --locked --project "$workspace"
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
For the global rules, see [navigation and verification](../../../../docs/guides/verification-and-navigation.md)
and the [executable contract](../../../../docs/reference/stage-executables.md).
Return to the [family](../../README.md).
