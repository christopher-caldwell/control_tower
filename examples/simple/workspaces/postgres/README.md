# Focused PostgreSQL ownership and recovery

Three ordinary Python stages show input ownership, a real PostgreSQL write, and
read-only inspection. The simple example supplies `psycopg[binary]`; this workspace
owns its schema and role files.
The database row is keyed by the active Control Tower UUID.

```text
001 Python seed -> data/record.json
002 Python/psycopg INSERT -> PostgreSQL row, then data/postgres_receipt.json
003 Python/psycopg SELECT -> data/postgres_inspection.json
```

## Setup and run

Use Unix tools, built Control Tower binaries, uv / Python 3.12, and PostgreSQL.
Put `initdb`, `pg_ctl`, `createdb`, and `psql` on PATH for the disposable-server
walkthrough. Keep the server and connection setting available throughout forward,
reverse, and recovery operations. Follow [example setup](../../README.md#setup-and-run),
then start from the copied simple example root.

```sh
workspace=workspaces/postgres

# Start a separate cluster as a regular user, with a private socket and no TCP listener.
export GALLERY_PG_DIR="$(mktemp -d /tmp/control-tower-gallery-pg.XXXXXX)"
mkdir "$GALLERY_PG_DIR/socket"
initdb -D "$GALLERY_PG_DIR/database" --auth-local=trust --auth-host=reject --encoding=UTF8 --locale=C
pg_ctl -D "$GALLERY_PG_DIR/database" -l "$GALLERY_PG_DIR/server.log" -w start \
  -o "-h '' -k '$GALLERY_PG_DIR/socket'"
createdb -h "$GALLERY_PG_DIR/socket" control_tower_gallery
export GALLERY_POSTGRES_DSN="host=$GALLERY_PG_DIR/socket dbname=control_tower_gallery"

# Schema creation belongs to setup, never a stage or verifier.
psql -X "$GALLERY_POSTGRES_DSN" -v ON_ERROR_STOP=1 -f "$workspace/schema.sql"
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 1
cat "$workspace/data/record.json"
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/postgres_receipt.json"
psql -X "$GALLERY_POSTGRES_DSN" -c 'TABLE public.gallery_records;'
# Visit each successive target separately to inspect the intermediate artifacts.
control-tower up --workspace "$workspace" --stage 3
cat "$workspace/data/postgres_inspection.json"
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
psql -X "$GALLERY_POSTGRES_DSN" -c 'TABLE public.gallery_records;'
# Stop only after reversal or explicit recovery is complete.
pg_ctl -D "$GALLERY_PG_DIR/database" -m fast -w stop
```

The cluster/log remain under `$GALLERY_PG_DIR` for inspection after stopping.
For an existing database you manage, set `GALLERY_POSTGRES_DSN` and run only schema
setup and workspace commands. Keep secrets outside repository files. Stages do not
start a server, create/drop schema, or drop a database. Control Tower's checkpoint
SQLite database is separate from this application PostgreSQL database.

## Ownership and reversal

Stage 002 owns one row keyed by `CONTROL_TOWER_UUID` and its receipt. It inserts
with `ON CONFLICT DO NOTHING`, checks the stored values in the same transaction,
commits, then writes the receipt. A retry accepts identical existing values and
refuses to overwrite conflicting data. Queries use bound parameters. Python
inspection/verifiers use read-only database transactions.

Reversing to stage 1 runs stage 002/down: delete only the active run's row,
commit the deletion, then remove its receipt. If SQL fails, the receipt remains.
Retrying deletion after a local cleanup failure is harmless. Reversing later stages
removes only their own inspection/output files and leaves the database row intact.
Reversing to baseline removes earlier input/tool outputs and clears the active UUID.
Unrelated rows, schema, checkpoint storage, installed environments, and logs remain.

## Failed stage 002: retry or abandon

`down` reverses accepted transitions. It is not a generic rollback mechanism for
partial effects produced by a mutation that failed before acceptance.

The database commit and receipt write are separate operations. To reproduce the
failure, start at baseline with the server running, schema prepared, and workspace
initialized. Run the following from the copied example root using the same `workspace`
variable as above:

```sh
control-tower up --workspace "$workspace" --stage 1
mkdir "$workspace/data/postgres_receipt.json"
```

Run the next command separately; its nonzero exit is intentional:

```sh
control-tower up --workspace "$workspace" --stage 2
```

The row may have committed. Control Tower remains at accepted stage 1, has no
pending verifier, and retains the UUID. Inspect the checkpoint and replace the
placeholder below with that retained UUID:

```sh
control-tower status --workspace "$workspace"
run_id='PASTE_THE_UUID_FROM_STATUS'
psql -X --no-password "$GALLERY_POSTGRES_DSN" --set=ON_ERROR_STOP=1 --set=run_id="$run_id" <<'SQL'
BEGIN READ ONLY;
SELECT run_id, record_id, label FROM public.gallery_records WHERE run_id = :'run_id'::uuid;
COMMIT;
SQL
```

**Retry:** remove the artificial local blocker and retry the failed stage:

```sh
rmdir "$workspace/data/postgres_receipt.json"
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/postgres_receipt.json"
```

The stage validates the existing run-owned row and finishes its receipt without
inserting a duplicate. If values conflict, inspect/repair the discrepancy before
retrying. Continue forward or reverse accepted transitions normally afterward.

**Abandon instead:** because stage 002 was never accepted, ordinary `down` will
not execute its reversal. Preserve the retained UUID until you have inspected and
explicitly deleted only that run's external effect:

```sh
psql -X --no-password "$GALLERY_POSTGRES_DSN" --set=ON_ERROR_STOP=1 --set=run_id="$run_id" <<'SQL'
BEGIN;
DELETE FROM public.gallery_records WHERE run_id = :'run_id'::uuid;
COMMIT;
SQL
# Remove only the artificial local blocker from this reproduction.
rmdir "$workspace/data/postgres_receipt.json"
control-tower down --workspace "$workspace" --stage 0
control-tower status --workspace "$workspace"
```

Earlier stages' `down` roles do not clean later unaccepted database effects.
Deleting `.control_tower/` or local output does not remove PostgreSQL rows.
After a verifier failure, repeating the same move retries only the observational
check without replaying the mutation. See [navigation guidance](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/verification-and-navigation.md#a-mutation-failure-is-not-a-verifier-failure).

## Optional integration coverage

From the Control Tower repository root:

```sh
./examples/test --family simple --postgres
```

Tests copy the complete example, start private disposable clusters, and cover traversal,
unrelated-row preservation, fresh UUIDs, missing schema, read-only verification,
verifier retry, conflicting-row refusal, receipt failure, retry, and abandonment.
Return to the [example](../../README.md).
