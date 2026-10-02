"""Real PostgreSQL ownership, read-only inspection, retry, and abandonment."""
import json
import os
from pathlib import Path
import shutil
import shlex
import tempfile
from uuid import uuid4

import psycopg
import pytest

from common import (
    RESULT, CONTEXT, NODE_INSPECTION, assert_node_owner, initialize, lock_contents,
    materialize_workspace, move, native_environment, run, status,
)

pytestmark = pytest.mark.postgres


@pytest.fixture(params=[
    pytest.param("simple", marks=pytest.mark.family_simple),
    pytest.param("py_capsule", marks=pytest.mark.family_py_capsule),
])
def postgres_workspace(request, tmp_path, cli_environment):
    family = request.param
    name = "postgres" if family == "simple" else "full-stack"
    sandbox, workspace = materialize_workspace(tmp_path, family, name, cli_environment)
    return sandbox, workspace, (2 if family == "simple" else 3), (3 if family == "simple" else 7)


@pytest.fixture
def postgres_dsn(cli_environment):
    for executable in ("initdb", "pg_ctl", "psql"):
        assert shutil.which(executable), "./test --postgres requires initdb, pg_ctl, and psql on PATH"
    # A short, private socket path avoids Unix socket limits in the long checkout
    # paths used by the other tests. Never connect to a user-managed database.
    server = Path(tempfile.mkdtemp(prefix="ct-gallery-pg-", dir="/tmp"))
    data = server / "database"
    socket = server / "socket"
    socket.mkdir()
    try:
        run(
            ["initdb", "-D", str(data), "--auth-local=trust", "--auth-host=reject", "--encoding=UTF8", "--locale=C"],
            cwd=server, env=cli_environment,
        )
        run(
            ["pg_ctl", "-D", str(data), "-l", str(server / "server.log"), "-w", "start", "-o",
             shlex.join(["-h", "", "-k", str(socket)])],
            cwd=server, env=cli_environment,
        )
        yield psycopg.conninfo.make_conninfo(host=str(socket), dbname="postgres")
    finally:
        if (data / "postmaster.pid").exists():
            # If stopping fails, retain the directory instead of removing a live DB.
            run(["pg_ctl", "-D", str(data), "-m", "fast", "-w", "stop"], cwd=server, env=cli_environment)
        shutil.rmtree(server)


def postgres_rows(dsn):
    with psycopg.connect(dsn) as connection:
        connection.read_only = True
        return {
            str(run_id): {"record_id": record_id, "label": label}
            for run_id, record_id, label in connection.execute(
                "SELECT run_id, record_id, label FROM public.gallery_records"
            )
        }


def prepare_postgres(workspace, dsn):
    # Same explicit SQL used in the opt-in instructions; no stage creates schema.
    with psycopg.connect(dsn) as connection:
        connection.execute((workspace / "schema.sql").read_text())


def test_postgres_workflow_reverses_only_its_external_row(tmp_path, cli_environment, postgres_dsn, postgres_workspace):
    sandbox, workspace, store_stage, final_stage = postgres_workspace
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    locks = lock_contents(sandbox)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", store_stage - 1)
    failed = move(sandbox, workspace, env, "up", store_stage, expected=1)
    assert "gallery_records" in failed.stderr + failed.stdout
    status(sandbox, workspace, env, store_stage - 1)
    assert not (workspace / "data/postgres_receipt.json").exists()
    prepare_postgres(workspace, postgres_dsn)
    unrelated_id = str(uuid4())
    unrelated = {"record_id": "background", "label": "KEEP ME"}
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("INSERT INTO public.gallery_records VALUES (%s, %s, %s)",
                           (unrelated_id, unrelated["record_id"], unrelated["label"]))
    move(sandbox, workspace, env, "up", store_stage)
    receipt = json.loads((workspace / "data/postgres_receipt.json").read_text())
    run_id = receipt["run_id"]
    assert receipt == {"run_id": run_id, **RESULT}
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    full_stack = store_stage == 3
    for target in range(store_stage + 1, final_stage + 1):
        move(sandbox, workspace, env, "up", target)
        status(sandbox, workspace, env, target)
        if target == store_stage + 1:
            filename = "node_inspection.json" if full_stack else "postgres_inspection.json"
            expected = {**NODE_INSPECTION, "runId": run_id} if full_stack else receipt
            assert json.loads((workspace / "data" / filename).read_text()) == expected
        elif target == 5:
            assert json.loads((workspace / "data/psql_command.json").read_text()) == {"run_id": run_id, "label_length": 14}
        elif target == 6:
            assert json.loads((workspace / "data/psql_file.json").read_text()) == {**receipt, "label_length": 14}
        elif target == 7:
            assert (workspace / "data/full_stack_complete").read_text() == "ok\n"
    before = {p.name: p.stat().st_mtime_ns for p in (workspace / "data").iterdir()}
    move(sandbox, workspace, env, "up", final_stage)
    assert before == {p.name: p.stat().st_mtime_ns for p in (workspace / "data").iterdir()}
    if full_stack:
        assert json.loads((workspace / "data/runtime_context.json").read_text()) == CONTEXT
        assert (sandbox / "tools/example_tool/.venv").is_dir()
        assert_node_owner(workspace / "stages/004-node-inspect", sandbox, env)
    # Database verifiers are read-only even when the external row is wrong.
    before = {p.name: p.read_bytes() for p in (workspace / "data").iterdir()}
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = 'WRONG' WHERE run_id = %s", (run_id,))
    role_env = dict(env, CONTROL_TOWER_WORKSPACE=str(workspace), CONTROL_TOWER_UUID=run_id)
    stages = [f"{store_stage:03d}-store-record"]
    stages += ["005-psql-command", "006-psql-file"] if full_stack else ["003-inspect-record"]
    for stage in stages:
        run([str(workspace / "stages" / stage / "verify-up")], cwd=workspace / "stages" / stage, env=role_env, expected=1)
    assert postgres_rows(postgres_dsn)[run_id]["label"] == "WRONG"
    assert {p.name: p.read_bytes() for p in (workspace / "data").iterdir()} == before
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = %s WHERE run_id = %s", (RESULT["label"], run_id))
    for target in range(final_stage - 1, -1, -1):
        move(sandbox, workspace, env, "down", target)
        status(sandbox, workspace, env, target)
        assert postgres_rows(postgres_dsn) == ({unrelated_id: unrelated, run_id: RESULT} if target >= store_stage else {unrelated_id: unrelated})
    assert list((workspace / "data").iterdir()) == []
    move(sandbox, workspace, env, "up", final_stage)
    second = json.loads((workspace / "data/postgres_receipt.json").read_text())
    assert second["run_id"] != run_id
    move(sandbox, workspace, env, "down", 0)
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}
    assert lock_contents(sandbox) == locks


@pytest.mark.parametrize("direction", ["up", "down"])
def test_postgres_verifier_retry_preserves_committed_effect(tmp_path, cli_environment, postgres_dsn, postgres_workspace, direction):
    sandbox, workspace, store_stage, final_stage = postgres_workspace
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", store_stage - 1 if direction == "up" else store_stage)
    stage = workspace / "stages" / f"{store_stage:03d}-store-record"
    verifier = stage / f"verify-{direction}"
    original = verifier.read_text()
    verifier.write_text(original.replace('import os\n', 'import os\nraise SystemExit("injected verifier failure")\n', 1))
    target = store_stage if direction == "up" else store_stage - 1
    move(sandbox, workspace, env, direction, target, expected=1)
    status(sandbox, workspace, env, store_stage - 1 if direction == "up" else store_stage, f"{direction} {store_stage}")
    committed = postgres_rows(postgres_dsn)
    assert len(committed) == (1 if direction == "up" else 0)
    verifier.write_text(original)
    (stage / direction).write_text('#!/bin/sh\nexit 99\n')
    move(sandbox, workspace, env, direction, target)
    status(sandbox, workspace, env, target)
    assert postgres_rows(postgres_dsn) == committed


def test_postgres_commit_survives_receipt_failure_and_retry(tmp_path, cli_environment, postgres_dsn, postgres_workspace):
    sandbox, workspace, store_stage, final_stage = postgres_workspace
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", store_stage - 1)
    # A directory at the output path forces a failure after the DB commit.
    receipt = workspace / "data/postgres_receipt.json"
    receipt.mkdir()
    move(sandbox, workspace, env, "up", store_stage, expected=1)
    status(sandbox, workspace, env, store_stage - 1)
    committed = postgres_rows(postgres_dsn)
    assert len(committed) == 1 and list(committed.values()) == [RESULT]
    run_id = next(iter(committed))
    assert f"UUID: {run_id}" in status(sandbox, workspace, env, store_stage - 1)
    # A retry must refuse to overwrite unexpected existing external data.
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = 'WRONG' WHERE run_id = %s", (run_id,))
    receipt.rmdir()
    move(sandbox, workspace, env, "up", store_stage, expected=1)
    assert postgres_rows(postgres_dsn)[run_id]["label"] == "WRONG"
    assert not receipt.exists()
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = %s WHERE run_id = %s", (RESULT["label"], run_id))
    move(sandbox, workspace, env, "up", store_stage)
    assert postgres_rows(postgres_dsn) == committed
    assert json.loads(receipt.read_text()) == {"run_id": run_id, **RESULT}
    assert f"UUID: {run_id}" in status(sandbox, workspace, env, store_stage)
    # Simulate an unavailable relation on down: receipt must survive failed SQL.
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("ALTER TABLE public.gallery_records RENAME TO saved_records")
    move(sandbox, workspace, env, "down", store_stage - 1, expected=1)
    assert receipt.exists()
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("ALTER TABLE public.saved_records RENAME TO gallery_records")
    move(sandbox, workspace, env, "down", store_stage - 1)
    assert postgres_rows(postgres_dsn) == {}
    assert not receipt.exists()


@pytest.mark.family_py_capsule
@pytest.mark.parametrize("target", [5, 6])
def test_psql_stages_stop_on_sql_error_and_retry(tmp_path, cli_environment, postgres_dsn, target):
    sandbox, workspace = materialize_workspace(tmp_path, "py_capsule", "full-stack", cli_environment)
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", target - 1)
    stage = workspace / "stages" / ("005-psql-command" if target == 5 else "006-psql-file")
    sql_source = stage / ("up" if target == 5 else "inspect.sql")
    original = sql_source.read_text()
    sql_source.write_text(original.replace("public.gallery_records", "public.missing_records"))
    before = postgres_rows(postgres_dsn)
    failed = move(sandbox, workspace, env, "up", target, expected=1)
    assert "missing_records" in failed.stdout + failed.stderr
    status(sandbox, workspace, env, target - 1)
    assert postgres_rows(postgres_dsn) == before
    sql_source.write_text(original)
    move(sandbox, workspace, env, "up", target)
    status(sandbox, workspace, env, target)
    artifact = workspace / "data" / ("psql_command.json" if target == 5 else "psql_file.json")
    assert json.loads(artifact.read_text())["label_length"] == 14
    # Matching-verifier retry must not execute the SQL role again.
    verifier = stage / "verify-down"
    verifier.write_text('#!/bin/sh\nexit 1\n')
    move(sandbox, workspace, env, "down", target - 1, expected=1)
    status(sandbox, workspace, env, target, f"down {target}")
    assert not artifact.exists()
    verifier.write_text(f'#!/bin/sh\nset -eu\ntest ! -e "$CONTROL_TOWER_WORKSPACE/data/{artifact.name}"\n')
    (stage / "down").write_text('#!/bin/sh\nexit 99\n')
    move(sandbox, workspace, env, "down", target - 1)
    status(sandbox, workspace, env, target - 1)
    assert postgres_rows(postgres_dsn) == before


def test_postgres_unaccepted_commit_is_abandoned_with_run_owned_cleanup(tmp_path, cli_environment, postgres_dsn, postgres_workspace):
    sandbox, workspace, store_stage, final_stage = postgres_workspace
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    unrelated_id = str(uuid4())
    unrelated = {"record_id": "background", "label": "KEEP ME"}
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute(
            "INSERT INTO public.gallery_records VALUES (%s, %s, %s)",
            (unrelated_id, unrelated["record_id"], unrelated["label"]),
        )
    move(sandbox, workspace, env, "up", store_stage - 1)
    checkpoint = status(sandbox, workspace, env, store_stage - 1)
    run_id = checkpoint.split("UUID: ", 1)[1].splitlines()[0]
    receipt = workspace / "data/postgres_receipt.json"
    receipt.mkdir()
    move(sandbox, workspace, env, "up", store_stage, expected=1)
    assert status(sandbox, workspace, env, store_stage - 1) == checkpoint
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    # Normal traversal cannot reverse the unaccepted stage. A sentinel proves it
    # is not executed; cleanup below is explicit user-land SQL, not a role call.
    (workspace / "stages" / f"{store_stage:03d}-store-record" / "down").write_text("#!/bin/sh\nexit 99\n")
    run(
        ["psql", "-X", "--no-password", postgres_dsn, "--set=ON_ERROR_STOP=1", f"--set=run_id={run_id}"],
        cwd=sandbox, env=env,
        input_text="BEGIN;\nDELETE FROM public.gallery_records WHERE run_id = :'run_id'::uuid;\nCOMMIT;\n",
    )
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}
    assert status(sandbox, workspace, env, store_stage - 1) == checkpoint
    receipt.rmdir()  # Only the artificial local blocker is removed here.
    move(sandbox, workspace, env, "down", 0)
    status(sandbox, workspace, env, 0)
    assert list((workspace / "data").iterdir()) == []
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}


@pytest.mark.family_py_capsule
def test_full_stack_native_boundaries_without_python(tmp_path, cli_environment, postgres_dsn):
    sandbox, workspace = materialize_workspace(tmp_path, "py_capsule", "full-stack", cli_environment)
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", 3)
    receipt = json.loads((workspace / "data/postgres_receipt.json").read_text())
    rows = postgres_rows(postgres_dsn)
    native = native_environment(sandbox, env, "node", "psql", "cat", "rm")
    project = sandbox / "pyproject.toml"
    original = project.read_text()
    project.write_text("[invalid Python metadata\n")
    for target in (4, 5, 6, 7):
        move(sandbox, workspace, native, "up", target)
        status(sandbox, workspace, native, target)
    assert json.loads((workspace / "data/node_inspection.json").read_text()) == {
        **NODE_INSPECTION, "runId": receipt["run_id"],
    }
    assert json.loads((workspace / "data/psql_command.json").read_text()) == {
        "run_id": receipt["run_id"], "label_length": 14,
    }
    assert json.loads((workspace / "data/psql_file.json").read_text()) == {**receipt, "label_length": 14}
    assert (workspace / "data/full_stack_complete").read_text() == "ok\n"
    for target in (6, 5, 4, 3):
        move(sandbox, workspace, native, "down", target)
        status(sandbox, workspace, native, target)
        assert postgres_rows(postgres_dsn) == rows
    assert {p.name for p in (workspace / "data").iterdir()} == {
        "record.json", "tool_result.json", "runtime_context.json", "postgres_receipt.json",
    }
    project.write_text(original)
    move(sandbox, workspace, env, "down", 0)
    assert postgres_rows(postgres_dsn) == {}
