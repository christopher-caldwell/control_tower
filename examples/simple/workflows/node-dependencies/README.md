# Ordinary Node dependencies

The simple example owns `package.json` and `package-lock.json`. Node
roles use normal module resolution to find example-owned `dayjs`; no role
installs packages. The timestamp is normalized to UTC before downstream inspection.

## Prerequisites and progression

Use a Unix shell, built Control Tower binaries, and Node and npm.
Follow [example setup](../../README.md#setup-and-run), then run these commands from the copied example root.

```text
001 Node seed (dayjs UTC) -> data/record.json
002 Node transform (dayjs UTC) -> data/node_inspection.json
```

Application data moves through the named files under `data/`; stdout is feedback.
Every stage has forward/reverse verification. Verifiers observe rather than repair
outputs. Dependencies belong to the containing example; each workflow owns its stage outputs.

## Run the workflow

```sh
workflow=workflows/node-dependencies
control-tower-db bootstrap-local "$workflow"
control-tower-db migrate-local "$workflow"
control-tower-db verify-local "$workflow"
# Use --stage 1, then each successive stage to inspect intermediate files.
control-tower up --workflow "$workflow" --stage 2
cat "$workflow/data/node_inspection.json"
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
