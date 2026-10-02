# Shell, Python, and Node

Shell persists input; Python parses its date and normalizes its values; Node
consumes those explicit files and produces a UTC-date inspection. This workspace
owns both language projects. Each runtime boundary is visible in the stage files.

## Prerequisites and progression

Use a Unix shell, built Control Tower binaries, and uv / Python 3.12 and Node / npm.
Start the commands below from the Control Tower repository root.

```text
001 shell seed -> data/record.json
002 Python normalize (dateutil) -> data/normalized.json and data/inspection.json
003 Node inspection (dayjs UTC) -> data/node_inspection.json
```

Application data moves through the named files under `data/`; stdout is feedback.
Every stage has forward/reverse verification. Verifiers observe rather than repair
outputs. This workspace has no dependency on sibling workspaces or family helpers.

## Run a disposable copy

```sh
cargo build --locked --workspace
export PATH="$PWD/target/debug:$PATH"
workspace=$(mktemp -d /tmp/control-tower-shell-python-node.XXXXXX)
cp -R examples/multi_language/workspaces/shell-python-node/. "$workspace/"
uv sync --locked --project "$workspace"
npm ci --prefix "$workspace" --ignore-scripts --no-audit --no-fund
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
# Use --stage 1, then each successive stage to inspect intermediate files.
control-tower up --workspace "$workspace" --stage 3
cat "$workspace/data/normalized.json"
cat "$workspace/data/node_inspection.json"
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
