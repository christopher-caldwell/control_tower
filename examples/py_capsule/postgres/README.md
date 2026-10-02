# Optional Postgres workflow

Use this workspace when you want to see stages make real PostgreSQL writes and
reverse them. The other workspaces and ordinary tests need no database server.
Control Tower still uses its own local SQLite checkpoint database; PostgreSQL holds
this scenario's application records. Python roles use the shared category project, which
includes `psycopg[binary]`; no workspace Python project or stage flags are needed.
The last two stages use shell and `psql` directly, with no Python runtime.

```text
001 call example_tool.normalize_record (PyCapsule)
    -> data/tool_result.json and data/runtime_context.json
002 read tool_result.json -> INSERT a run-owned PostgreSQL row
    -> data/postgres_receipt.json after the commit
003 SELECT the row -> data/postgres_inspection.json
004 psql --command "SQL string" -> data/psql_command.json (label length)
005 psql --file ./inspect.sql -> data/psql_file.json (record and label length)
```

## Try it

Follow [category setup](../README.md#setup-and-run) from the repository root
and put `target/debug` on PATH. Install PostgreSQL separately and put `initdb`, `pg_ctl`, `createdb`,
and `psql` on PATH. The following commands start a separate local cluster with a
private Unix socket and no TCP listener; they do not use an existing database.
Run them in the same shell as a regular user:

```sh
export GALLERY_PG_DIR="$(mktemp -d /tmp/control-tower-gallery-pg.XXXXXX)"
mkdir "$GALLERY_PG_DIR/socket"
initdb -D "$GALLERY_PG_DIR/database" --auth-local=trust --auth-host=reject --encoding=UTF8 --locale=C
pg_ctl -D "$GALLERY_PG_DIR/database" -l "$GALLERY_PG_DIR/server.log" -w start \
  -o "-h '' -k '$GALLERY_PG_DIR/socket'"
createdb -h "$GALLERY_PG_DIR/socket" control_tower_gallery
export GALLERY_POSTGRES_DSN="host=$GALLERY_PG_DIR/socket dbname=control_tower_gallery"

psql -X "$GALLERY_POSTGRES_DSN" -v ON_ERROR_STOP=1 -f examples/py_capsule/postgres/schema.sql
control-tower-db bootstrap-local examples/py_capsule/postgres
control-tower-db migrate-local examples/py_capsule/postgres
control-tower-db verify-local examples/py_capsule/postgres
control-tower up --workspace examples/py_capsule/postgres --stage 1
cat examples/py_capsule/postgres/data/tool_result.json
control-tower up --workspace examples/py_capsule/postgres --stage 2
cat examples/py_capsule/postgres/data/postgres_receipt.json
psql "$GALLERY_POSTGRES_DSN" -c 'TABLE public.gallery_records;'
control-tower up --workspace examples/py_capsule/postgres --stage 3
cat examples/py_capsule/postgres/data/postgres_inspection.json
control-tower up --workspace examples/py_capsule/postgres --stage 4
cat examples/py_capsule/postgres/data/psql_command.json
control-tower up --workspace examples/py_capsule/postgres --stage 5
cat examples/py_capsule/postgres/data/psql_file.json
control-tower status --workspace examples/py_capsule/postgres
control-tower down --workspace examples/py_capsule/postgres --stage 0
psql "$GALLERY_POSTGRES_DSN" -c 'TABLE public.gallery_records;'

pg_ctl -D "$GALLERY_PG_DIR/database" -m fast -w stop
```

The cluster and log remain at `$GALLERY_PG_DIR` for inspection after stopping.
To use a database you already manage instead, set `GALLERY_POSTGRES_DSN` to its
connection string and run the explicit schema setup and workspace commands. The
schema is demonstration setup, not a migration framework. No role creates or drops
schema, and `down` never drops the table or database. Keep connection secrets out
of repository files; only the database connection setting is required here.

## Ownership and recovery

Stages 004 and 005 demonstrate two ordinary psql entry points. Stage 004 passes a
SQL string with `--command`. It supplies the run UUID through a session setting
and reads it with `current_setting()`, keeping values out of the SQL string.
Stage 005 runs the separate [inspect.sql](stages/005-psql-file/inspect.sql) file
using `--file`. It supplies a psql variable and the file uses `:'run_id'` to quote
it as a SQL literal. Both select the row for the active run and save JSON produced
by PostgreSQL. There is no application-language code in either stage.

Both psql connections use read-only transactions, `-X` to ignore user startup
scripts, and `ON_ERROR_STOP=1` to report SQL failures to Control Tower. Their
verifiers run read-only SQL and compare the database result with the saved output.
The file is resolved from the stage working directory. These stages assume `psql`
is installed; they neither install dependencies nor start a server.

Stage 002 owns one row keyed by `CONTROL_TOWER_UUID`, plus its local receipt.
`up` inserts with `ON CONFLICT DO NOTHING`, then checks the stored values in the same
transaction. A repeat accepts the same row and rejects unexpected existing values.
SQL uses parameters rather than interpolating record values. Inspection and
verifiers use read-only database transactions. Each CLI command receives the same
run UUID until reversal reaches baseline; a subsequent run gets a new UUID.

### Failed stage 002: retry or abandon

Database commit and receipt writing are separate operations. This exact failure can
leave an external effect without an accepted transition:

```text
stage 001 accepted
stage 002 INSERT commits, then postgres_receipt.json writing fails
Control Tower: completed stage 1, no pending verifier, retained run UUID
PostgreSQL: the run-owned row exists
```

To reproduce, start with an initialized workspace at baseline, the schema prepared,
and the server/connection still available as above. From the repository root:

```sh
workspace=examples/py_capsule/postgres
control-tower up --workspace "$workspace" --stage 1
mkdir "$workspace/data/postgres_receipt.json"
```

Run the next command separately; its nonzero exit is intentional:

```sh
control-tower up --workspace "$workspace" --stage 2
```

Inspect the checkpoint and copy the UUID printed by `status` into `run_id`. The
placeholder below must be replaced with that retained UUID, never another run's ID:

```sh
control-tower status --workspace "$workspace"
run_id='PASTE_THE_UUID_FROM_STATUS'
psql -X --no-password "$GALLERY_POSTGRES_DSN" --set=ON_ERROR_STOP=1 --set=run_id="$run_id" <<'SQL'
BEGIN READ ONLY;
SELECT run_id, record_id, label FROM public.gallery_records WHERE run_id = :'run_id'::uuid;
COMMIT;
SQL
```

**Continue the run:** repair the local output problem and retry stage 002. For the
injected directory failure:

```sh
rmdir "$workspace/data/postgres_receipt.json"
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/postgres_receipt.json"
```

The stage recognizes and validates the existing run-owned row, then writes the
receipt. It neither inserts a duplicate nor silently overwrites unexpected values.
If stored values differ, inspect/repair that discrepancy before retrying. You can
then continue to stage 5 or reverse accepted transitions normally.

**Abandon the run instead:** stage 002 was never accepted, so normal `down` traversal
will not run stage 002/down. Inspect the row as above, then explicitly remove only
this run's effect with this example-owned SQL procedure:

```sh
psql -X --no-password "$GALLERY_POSTGRES_DSN" --set=ON_ERROR_STOP=1 --set=run_id="$run_id" <<'SQL'
BEGIN;
DELETE FROM public.gallery_records WHERE run_id = :'run_id'::uuid;
COMMIT;
SQL
# Remove only the artificial receipt-path blocker, if you used the reproduction.
rmdir "$workspace/data/postgres_receipt.json"
control-tower down --workspace "$workspace" --stage 0
control-tower status --workspace "$workspace"
```

The SQL cleanup preserves unrelated rows. Stage 001/down removes its own tool
outputs; it does not clean stage 002's database effect. Keep the retained UUID until
external cleanup is complete: successful settlement at baseline clears it.
Deleting `.control_tower/` or local output files does not delete PostgreSQL rows.
This is user-land recovery, not a new role, runner convention, or rollback protocol.
`down` reverses accepted transitions; recovery from partial effects of a failed
mutation belongs to the workflow that owns those effects. See the maintained
[mutation-failure guidance](../../../docs/guides/verification-and-navigation.md#a-mutation-failure-is-not-a-verifier-failure).

### Reversing accepted stages

`down --stage 4` removes only stage 005's SQL-file output, and `down --stage 3`
also removes stage 004's SQL-string output. Both leave the database row intact.
`down --stage 2` also removes stage 003's local inspection. `down --stage 1` deletes
the actual database row for this run, commits that deletion, then removes its
receipt. If SQL fails, the receipt stays. If local deletion fails after the commit,
retrying the run-scoped database deletion is harmless. `down --stage 0` also removes
stage 001's local outputs. Other runs' rows, unrelated rows, schema, checkpoint DB,
and PyCapsule diagnostics remain. This is explicit compensation for this particular
insert, not a claim that local cleanup compensates arbitrary external calls.

A failing psql command can leave an empty or partial local JSON file because shell
redirection happens before the query. No database mutation has occurred in those
read-only stages. Fix the query, connection, or output path and repeat the same
`up`; it overwrites that stage's output. Reversal removes only its output file.

After an up/down verifier fails, repeating the same move retries only verification
without repeating the committed operation. Keep the connection setting and server
available throughout forward/reverse commands and recovery.

## Opt-in integration test

```sh
./examples/py_capsule/test --postgres
```

This starts/stops disposable PostgreSQL clusters through `initdb`/`pg_ctl` and runs
real Control Tower CLIs against copied workspaces. It never uses your connection
setting or application databases. Coverage includes row read/write/reversal,
unrelated-row preservation, a second run, read-only verification, verifier-only
retries, missing schema, and database-commit/local-receipt failure with retry and abandonment. The psql stages
also run with no Python/uv on PATH and invalid category Python metadata. SQL failures
and reverse-verifier retries are checked for both string and file execution.
Ordinary `./examples/py_capsule/test`
skips these tests and never starts PostgreSQL. Local validation used PostgreSQL 18.3.
