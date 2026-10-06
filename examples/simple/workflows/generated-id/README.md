# Application-generated ID handoff

This optional two-stage workflow creates a row in an **application SQLite database**, carries its generated integer ID in an author-owned JSON file, and changes/restores that same row. Control Tower still launches ordinary executables; it does not parse the ID from stdout or manage the handoff.

You need the [usual build prerequisites](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/getting-started.md#prerequisites) **plus Python 3 with its standard-library `sqlite3` module**. No Python packages, separate SQLite installation, server, or credentials are needed. Python is a prerequisite for this scenario; the [original UUID-file quickstart](https://github.com/christopher-caldwell/control_tower/blob/main/README.md#try-the-three-stage-example) does not need it.

## Prepare the workflow

Follow [simple example setup](../../README.md#setup-and-run), then run from the copied example root. This scenario needs no uv or npm setup:

```sh
python3 --version

workflow=workflows/generated-id
printf 'Example workflow: %s\n' "$workflow"

control-tower db bootstrap-local --workflow "$workflow"
control-tower db migrate-local --workflow "$workflow"
control-tower db verify-local --workflow "$workflow"
control-tower status --workflow "$workflow"
```

Status initially reports baseline 0. These commands prepare only Control Tower's `.control_tower/state.sqlite3`. Stage 1/up creates `data/application.sqlite3` and its `fixture` table, inserts a row, then writes its SQLite-generated ID to `data/record.json`. The ID is independent of `CONTROL_TOWER_UUID`.

## Walk 0 → 1 → 2 → 1 → 2 → 0

```sh
control-tower up --workflow "$workflow" --stage 1
cat "$workflow/data/record.json"
handoff="$(cat "$workflow/data/record.json")"

control-tower up --workflow "$workflow" --stage 2
test "$(cat "$workflow/data/record.json")" = "$handoff"
control-tower down --workflow "$workflow" --stage 1
test "$(cat "$workflow/data/record.json")" = "$handoff"
control-tower up --workflow "$workflow" --stage 2
test "$(cat "$workflow/data/record.json")" = "$handoff"

control-tower down --workflow "$workflow" --stage 0
test ! -e "$workflow/data/record.json"
control-tower status --workflow "$workflow"
```

Each mutation/check prints the ID and observed value. The JSON stays identical through stages 1 and 2 and their reversals; it is removed only by stage 1/down.

| Accepted position | Application fixture |
| --- | --- |
| 0, before first up | No application database or handoff yet. |
| 1 | Identified row has value `initial`. |
| 2 | Same row has value `changed`. |
| 1, after down | Same row is restored to `initial`. |
| 0, after final down | No fixture rows or handoff; Control Tower UUID is cleared. |

The application database/schema and Control Tower database may remain. Their presence is not a remaining fixture row. Inspect the retained application database read-only:

```sh
python3 - "$workflow" <<'PY'
from contextlib import closing
from pathlib import Path
import sqlite3, sys
database = Path(sys.argv[1]) / "data/application.sqlite3"
with closing(sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True)) as db:
    count = db.execute("SELECT COUNT(*) FROM fixture").fetchone()[0]
if count != 0:
    sys.exit("fixture rows remain")
print("Fixture rows:", count)
PY
```

## Fail a check, repair it, retry only the check

Prepare another fresh copy of the complete simple example using its README, then run from that example root. This exercise logs the stage 2 mutation so a successful retry cannot silently repeat it:

```sh
workflow=workflows/generated-id
control-tower db bootstrap-local --workflow "$workflow"
control-tower db migrate-local --workflow "$workflow"
control-tower db verify-local --workflow "$workflow"

stage="$workflow/stages/002-change-record"
cp "$stage/up" "$workflow/up.original"
cat > "$stage/up" <<'SH'
#!/bin/sh
set -eu
printf '%s %s\n' "$CONTROL_TOWER_STAGE" "$CONTROL_TOWER_ROLE" >> "$CONTROL_TOWER_WORKFLOW/mutation-calls"
exec "$CONTROL_TOWER_WORKFLOW/up.original"
SH
chmod +x "$stage/up" "$workflow/up.original"
cp "$stage/verify-up" "$workflow/verify-up.original"
printf '#!/bin/sh\nexit 23\n' > "$stage/verify-up"
chmod +x "$stage/verify-up"
```

Run this separately. **Its nonzero exit is expected**:

```sh
control-tower up --workflow "$workflow" --stage 2
```

Stage 2/up has changed the row, but recorded completed position stays 1 with pending up 2. The movement prints commands for retrying that check or reversing the active stage. Repair and retry:

```sh
control-tower status --workflow "$workflow"
cp "$workflow/verify-up.original" "$stage/verify-up"
chmod +x "$stage/verify-up"
control-tower up --workflow "$workflow" --stage 2
test "$(cat "$workflow/mutation-calls")" = '2 up'
control-tower status --workflow "$workflow"
control-tower down --workflow "$workflow" --stage 0
control-tower status --workflow "$workflow"
```

Only stage 2/verify-up starts on retry. The single `2 up` log line proves the successful mutation ran once. Final status reports baseline 0 and no UUID.

## Authoring boundary and limitations

The eight executable role files are thin entry points into `support/fixture.py`. They use the documented stage working directory/environment. All checks open the application DB with SQLite `mode=ro`; they never initialize a missing DB/schema, create data, or repair a wrong value. Missing data/schema produces a nonzero error, including with Python optimization enabled. Only stage 1/up initializes the application fixture.

This is a small local fixture, not an SDK or managed context format. SQLite mutation, JSON publication, and Control Tower checkpoint writes are separate operations. If a mutation partially fails, inspect the row and handoff yourself; `down 0` is not guaranteed cleanup after a failed first mutation. `status` reports bookkeeping and does not query the application row. Finish/clean the run before structural stage edits. See [authoring](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/creating-a-workflow.md) and [navigation](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/verification-and-navigation.md).
