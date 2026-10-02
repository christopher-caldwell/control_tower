"""Run the gallery through real CLIs, uv shebangs, and the PyCapsule child."""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import tempfile
from uuid import uuid4

import psycopg
import pytest

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = ROOT.parents[1]
PATTERNS = [
    ("repo-default-python", 3),
    ("isolated-python-stage", 3),
    ("polyglot", 4),
]
RESULT = {"record_id": "123", "label": "EXAMPLE RECORD"}
RECORD = {"id": 123, "label": "  example record  ", "created_at": "2026-01-02T03:04:05+00:00"}
CONTEXT = {
    "session": {"values": {"last_record_id": "123"}, "events": []},
    "conversation": {"metadata": {"last_tool": "normalize_record"}},
}


def run(command, *, cwd, env, expected=0, input_text=None):
    result = subprocess.run(
        command, cwd=cwd, env=env, capture_output=True, text=True, timeout=180, input=input_text
    )
    assert result.returncode == expected, result.stdout + result.stderr
    return result


def materialize_pattern(tmp_path, name):
    sandbox = tmp_path / "repo with spaces and 'quotes'" / "examples/py_capsule"
    sandbox.mkdir(parents=True)
    ignore = shutil.ignore_patterns(".venv", "__pycache__", "*.egg-info", ".control_tower", "data")
    shutil.copytree(ROOT / "tools/example_tool", sandbox / "tools/example_tool", ignore=ignore)
    workspace = sandbox / name
    shutil.copytree(ROOT / name, workspace, ignore=ignore)
    for filename in ("pyproject.toml", "uv.lock", ".python-version", "package.json", "package-lock.json"):
        shutil.copy2(ROOT / filename, sandbox / filename)
    if name == "polyglot":
        run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"], cwd=sandbox, env=os.environ)
    return sandbox, workspace


@pytest.fixture
def cli_environment():
    for executable in ("control-tower", "control-tower-db"):
        assert (REPOSITORY / "target/debug" / executable).is_file(), "run cargo build --locked --workspace from the repository root before tests"
    for executable in ("uv", "node", "npm"):
        assert shutil.which(executable), f"{executable} is required for the complete gallery"
    env = dict(os.environ)
    for variable in (
        "PYTHONPATH", "UV_PROJECT", "UV_PROJECT_ENVIRONMENT", "UV_NO_SYNC",
        "VIRTUAL_ENV", "UV_LOCKED", "UV_FROZEN",
    ):
        env.pop(variable, None)
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    env["PATH"] = str(REPOSITORY / "target/debug") + os.pathsep + env["PATH"]
    return env


def initialize(sandbox, workspace, env):
    for operation in ("bootstrap-local", "migrate-local", "verify-local"):
        run(["control-tower-db", operation, str(workspace)], cwd=sandbox, env=env)


def move(sandbox, workspace, env, direction, stage, *, expected=0):
    return run(
        ["control-tower", direction, "--workspace", str(workspace), "--stage", str(stage)],
        cwd=sandbox, env=env, expected=expected,
    )


def status(sandbox, workspace, env, completed, pending=None):
    text = run(["control-tower", "status", "--workspace", str(workspace)], cwd=sandbox, env=env).stdout
    assert f"Completed stage: {completed if completed else 'baseline (0)'}" in text
    if pending:
        assert f"Pending verification: {pending}" in text
    else:
        assert "Pending verification:" not in text
    if not completed and not pending:
        assert "UUID: not created" in text
    return text


def assert_artifacts(workspace, name, completed):
    expected = {}
    if completed >= 1:
        expected["record.json"] = RECORD
    if completed >= 2:
        expected.update({"tool_result.json": RESULT, "runtime_context.json": CONTEXT})
    if completed >= 3:
        if name == "polyglot":
            expected["node_inspection.json"] = {
                "runtime": "node", "recordId": "123", "label": "EXAMPLE RECORD", "createdDate": "2026-01-02",
            }
        else:
            expected["inspection.json"] = {"result": RESULT, "last_record_id": "123", "last_tool": "normalize_record"}
    if completed == 4:
        expected["polyglot_complete"] = "ok\n"
    data = workspace / "data"
    assert ({path.name for path in data.iterdir()} if data.exists() else set()) == set(expected)
    for filename, value in expected.items():
        content = (data / filename).read_text()
        assert (content if isinstance(value, str) else json.loads(content)) == value


def assert_environment_ownership(sandbox, workspace, env, name):
    if name == "polyglot":
        projects = [workspace / "stages/002-call-tool"]
    else:
        projects = [workspace / "stages" / stage for stage in ("001-seed", "002-call-tool", "003-inspect")]
    for project in projects:
        # Use the same nearest-project discovery as the shebangs, from the stage cwd.
        info = json.loads(run(
            ["uv", "run", "python", "-c",
             "import sys, json; from importlib.metadata import distributions; "
             "print(json.dumps({'prefix': sys.prefix, 'packages': sorted(d.metadata['Name'] for d in distributions())}))"],
            cwd=project, env=env,
        ).stdout)
        owner = project if (project / "pyproject.toml").exists() else sandbox
        assert Path(info["prefix"]).resolve() == (owner / ".venv").resolve()
        expected_tool = True
        if name == "isolated-python-stage" and project.name == "003-inspect":
            expected_tool = False
            assert "jsonschema" in info["packages"]
            # The actual role uses this dependency, unavailable from the category.
            unavailable = run(
                [str(sandbox / ".venv/bin/python"), "-c", "import jsonschema"],
                cwd=sandbox, env=env, expected=1,
            )
            assert "ModuleNotFoundError" in unavailable.stderr
        else:
            assert "jsonschema" not in info["packages"]
        assert ("example-tool" in info["packages"]) == expected_tool
        assert ("python-dateutil" in info["packages"]) == expected_tool
    if name == "polyglot":
        module = run(["node", "-p", 'require.resolve("dayjs")'], cwd=workspace / "stages/003-node-inspect", env=env).stdout.strip()
        assert Path(module).is_relative_to(sandbox / "node_modules/dayjs")
    # PyCapsule owns a second environment at the tool root, regardless of caller.
    assert (sandbox / "tools/example_tool/.venv").is_dir()


@pytest.mark.parametrize(("name", "target"), PATTERNS)
def test_workspace_pattern_runs_through_real_control_tower(tmp_path, cli_environment, name, target):
    sandbox, workspace = materialize_pattern(tmp_path, name)
    env = cli_environment
    lockfiles = [sandbox / "uv.lock", sandbox / "package-lock.json"] + list((sandbox / "tools").rglob("uv.lock")) + list(workspace.rglob("uv.lock"))
    locks_before = {path: path.read_bytes() for path in lockfiles}
    initialize(sandbox, workspace, env)
    status(sandbox, workspace, env, 0)
    for stage in range(1, target + 1):
        move(sandbox, workspace, env, "up", stage)
        status(sandbox, workspace, env, stage)
        assert_artifacts(workspace, name, stage)
    # Reaching the current stage is observational; it must not replay a tool call.
    before = {p.name: p.stat().st_mtime_ns for p in (workspace / "data").iterdir()}
    move(sandbox, workspace, env, "up", target)
    assert before == {p.name: p.stat().st_mtime_ns for p in (workspace / "data").iterdir()}
    assert_environment_ownership(sandbox, workspace, env, name)
    for stage in range(target - 1, -1, -1):
        move(sandbox, workspace, env, "down", stage)
        status(sandbox, workspace, env, stage)
        assert_artifacts(workspace, name, stage)
    # A second run gets a clean runtime, and an independent process can reverse it.
    move(sandbox, workspace, env, "up", target)
    assert_artifacts(workspace, name, target)
    move(sandbox, workspace, env, "down", 0)
    assert_artifacts(workspace, name, 0)
    assert {path: path.read_bytes() for path in lockfiles} == locks_before


def test_new_ticket_needs_only_executable_python_roles(tmp_path, cli_environment):
    sandbox, _ = materialize_pattern(tmp_path, "repo-default-python")
    workspace = sandbox / "ticket_123"
    normalize = workspace / "stages/001-normalize"
    inspect = workspace / "stages/002-inspect"
    # Author a new ticket after repo setup: no project files, path selection, or
    # lock-policy flags. Only the executable roles describe the investigation.
    common = '''#!/usr/bin/env -S uv run python
import json
import os
from pathlib import Path
workspace = Path(os.environ["CONTROL_TOWER_WORKSPACE"])
data = workspace / "data"
'''
    roles = {
        normalize: {
            "up": common + '''from example_tool import normalize_record
result = normalize_record(record_id="123", label="  example record  ")
data.mkdir(parents=True, exist_ok=True)
(data / "normalized.json").write_text(json.dumps({"value": result.value, "runtime": result.runtime_export}))
''',
            "verify-up": common + '''saved = json.loads((data / "normalized.json").read_text())
if saved["value"] != {"record_id": "123", "label": "EXAMPLE RECORD"}:
    raise SystemExit("unexpected normalized record")
if saved["runtime"]["conversation"]["metadata"]["last_tool"] != "normalize_record":
    raise SystemExit("missing capsule runtime export")
''',
            "down": common + '(data / "normalized.json").unlink(missing_ok=True)\n',
            "verify-down": common + '''if (data / "normalized.json").exists():
    raise SystemExit("normalized output remains")
''',
        },
        inspect: {
            "up": common + '''saved = json.loads((data / "normalized.json").read_text())
(data / "inspection.txt").write_text(saved["value"]["label"] + "\\n")
''',
            "verify-up": common + '''if (data / "inspection.txt").read_text() != "EXAMPLE RECORD\\n":
    raise SystemExit("unexpected downstream inspection")
''',
            "down": common + '(data / "inspection.txt").unlink(missing_ok=True)\n',
            "verify-down": common + '''if (data / "inspection.txt").exists():
    raise SystemExit("inspection remains")
''',
        },
    }
    for stage, sources in roles.items():
        stage.mkdir(parents=True)
        for role, source in sources.items():
            path = stage / role
            path.write_text(source)
            path.chmod(0o755)
    env = cli_environment
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", 1)
    saved = (workspace / "data/normalized.json").read_bytes()
    assert json.loads(saved) == {"value": RESULT, "runtime": CONTEXT}
    assert (sandbox / "tools/example_tool/.venv").is_dir()
    move(sandbox, workspace, env, "up", 2)
    status(sandbox, workspace, env, 2)
    assert (workspace / "data/inspection.txt").read_text() == "EXAMPLE RECORD\n"
    move(sandbox, workspace, env, "down", 1)
    assert (workspace / "data/normalized.json").read_bytes() == saved
    assert not (workspace / "data/inspection.txt").exists()
    move(sandbox, workspace, env, "down", 0)
    status(sandbox, workspace, env, 0)
    assert list((workspace / "data").iterdir()) == []
    assert not list(workspace.rglob("pyproject.toml"))
    assert not list(workspace.rglob("uv.lock"))
    assert not list(workspace.rglob(".venv"))
    info = run(
        ["uv", "run", "python", "-c", "import sys, example_tool; print(sys.prefix); print(example_tool.__file__)"],
        cwd=normalize, env=env,
    ).stdout.splitlines()
    assert Path(info[0]).resolve() == (sandbox / ".venv").resolve()
    assert Path(info[1]).is_relative_to(sandbox / "tools/example_tool/src")


def test_node_and_shell_work_without_a_usable_python_project(tmp_path, cli_environment):
    name = "polyglot"
    sandbox, workspace = materialize_pattern(tmp_path, name)
    env = cli_environment
    initialize(sandbox, workspace, env)
    # A PATH with no uv/python, and invalid category metadata, make accidental Python
    # coupling fail rather than silently inheriting the developer's environment.
    bin_dir = sandbox / "native-bin"
    bin_dir.mkdir()
    for executable in ("control-tower", "node", "mkdir", "cat", "rm"):
        os.symlink(shutil.which(executable, path=env["PATH"]), bin_dir / executable)
    native_env = dict(env, PATH=str(bin_dir))
    project = sandbox / "pyproject.toml"
    original = project.read_text()
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "up", 1)
    assert_artifacts(workspace, name, 1)
    project.write_text(original)
    move(sandbox, workspace, env, "up", 2)
    project.write_text("[invalid project metadata\n")
    for stage in (3, 4):
        move(sandbox, workspace, native_env, "up", stage)
        assert_artifacts(workspace, name, stage)
    for stage in (3, 2):
        move(sandbox, workspace, native_env, "down", stage)
        assert_artifacts(workspace, name, stage)
    project.write_text(original)
    move(sandbox, workspace, env, "down", 1)
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "down", 0)
    status(sandbox, workspace, native_env, 0)
    assert_artifacts(workspace, name, 0)


@pytest.mark.parametrize("direction", ["up", "down"])
def test_verifier_retry_does_not_repeat_mutation(tmp_path, cli_environment, direction):
    name = "repo-default-python"
    sandbox, workspace = materialize_pattern(tmp_path, name)
    env = cli_environment
    initialize(sandbox, workspace, env)
    stage = workspace / "stages/002-call-tool"
    if direction == "down":
        move(sandbox, workspace, env, "up", 2)
    # Fail just the observational verifier. All role source changes are disposable.
    verifier = stage / f"verify-{direction}"
    original = verifier.read_text()
    verifier.write_text(original.replace('import os\n', 'import os\nraise SystemExit("injected verifier failure")\n', 1))
    target = 2 if direction == "up" else 1
    move(sandbox, workspace, env, direction, target, expected=1)
    status(sandbox, workspace, env, 1 if direction == "up" else 2, f"{direction} 2")
    assert_artifacts(workspace, name, target)
    verifier.write_text(original)
    # A replay would now fail, proving recovery only executes the verifier.
    (stage / direction).write_text('#!/bin/sh\nexit 99\n')
    move(sandbox, workspace, env, direction, target)
    status(sandbox, workspace, env, target)
    assert_artifacts(workspace, name, target)


@pytest.mark.parametrize("owner", ["category", "tool", "isolated-stage"])
def test_strict_validation_rejects_stale_dependency_lock(tmp_path, cli_environment, owner):
    name = "isolated-python-stage"
    sandbox, workspace = materialize_pattern(tmp_path, name)
    # Bootstrap/preceding tests cache these dependencies. Offline resolution avoids
    # registry refreshes while checking stale-lock rejection, including the child.
    env = dict(cli_environment, UV_LOCKED="1", UV_OFFLINE="1")
    initialize(sandbox, workspace, env)
    project = {
        "category": sandbox / "pyproject.toml",
        "tool": sandbox / "tools/example_tool/pyproject.toml",
        "isolated-stage": workspace / "stages/003-inspect/pyproject.toml",
    }[owner]
    if owner != "category":
        move(sandbox, workspace, env, "up", 1 if owner == "tool" else 2)
    if owner == "tool":
        # Refresh only the caller's lock after changing tool-only group metadata.
        # The child lock stays stale, proving validation reaches PyCapsule's uv.
        project.write_text(project.read_text() + '\n[dependency-groups]\ndev = ["pytest>=8,<10"]\n')
        run(["uv", "lock", "--project", str(sandbox)], cwd=sandbox, env=dict(cli_environment, UV_OFFLINE="1"))
        run(["uv", "lock", "--check", "--project", str(sandbox)], cwd=sandbox, env=env)
    else:
        project.write_text(project.read_text().replace('requires-python = ">=3.11"', 'requires-python = ">=3.12"'))
    lock = project.with_name("uv.lock")
    before = lock.read_bytes()
    # The same locked sync used by setup must reject stale metadata too.
    rejected = run(["uv", "sync", "--locked", "--project", str(project.parent)], cwd=sandbox, env=env, expected=1)
    assert "lockfile" in rejected.stdout + rejected.stderr
    failed = move(sandbox, workspace, env, "up", {"category": 1, "tool": 2, "isolated-stage": 3}[owner], expected=1)
    assert "lockfile" in failed.stdout + failed.stderr
    assert lock.read_bytes() == before
    assert_artifacts(workspace, name, {"category": 0, "tool": 1, "isolated-stage": 2}[owner])


POSTGRES_ONLY = pytest.mark.skipif(
    os.environ.get("GALLERY_TEST_POSTGRES") != "1", reason="opt in with ./test --postgres"
)


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


@POSTGRES_ONLY
def test_postgres_workflow_reverses_only_its_external_row(tmp_path, cli_environment, postgres_dsn):
    sandbox, workspace = materialize_pattern(tmp_path, "postgres")
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", 1)
    # Missing schema must fail rather than running migrations inside a stage.
    failed = move(sandbox, workspace, env, "up", 2, expected=1)
    assert "gallery_records" in failed.stderr + failed.stdout
    assert not (workspace / "data/postgres_receipt.json").exists()
    prepare_postgres(workspace, postgres_dsn)
    unrelated_id = str(uuid4())
    unrelated = {"record_id": "background", "label": "KEEP ME"}
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute(
            "INSERT INTO public.gallery_records VALUES (%s, %s, %s)",
            (unrelated_id, unrelated["record_id"], unrelated["label"]),
        )
    move(sandbox, workspace, env, "up", 2)
    receipt = json.loads((workspace / "data/postgres_receipt.json").read_text())
    run_id = receipt["run_id"]
    assert receipt == {"run_id": run_id, **RESULT}
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    move(sandbox, workspace, env, "up", 3)
    status(sandbox, workspace, env, 3)
    assert json.loads((workspace / "data/postgres_inspection.json").read_text()) == receipt
    assert json.loads((workspace / "data/runtime_context.json").read_text()) == CONTEXT
    # The SQL string/file stages use native tools with no usable Python runtime.
    native_bin = sandbox / "native-bin"
    native_bin.mkdir()
    for executable in ("control-tower", "psql", "cat", "rm"):
        os.symlink(shutil.which(executable, path=env["PATH"]), native_bin / executable)
    native_env = dict(env, PATH=str(native_bin))
    project = sandbox / "pyproject.toml"
    original_project = project.read_text()
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "up", 4)
    assert json.loads((workspace / "data/psql_command.json").read_text()) == {"run_id": run_id, "label_length": 14}
    move(sandbox, workspace, native_env, "up", 5)
    status(sandbox, workspace, native_env, 5)
    assert json.loads((workspace / "data/psql_file.json").read_text()) == {**receipt, "label_length": 14}
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    project.write_text(original_project)
    # Verifiers are read-only, including when the external row is incorrect.
    before = {p.name: p.read_bytes() for p in (workspace / "data").iterdir()}
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = 'WRONG' WHERE run_id = %s", (run_id,))
    role_env = dict(env, CONTROL_TOWER_WORKSPACE=str(workspace), CONTROL_TOWER_UUID=run_id)
    for stage in ("002-store-record", "003-inspect-record", "004-psql-command", "005-psql-file"):
        run([str(workspace / "stages" / stage / "verify-up")], cwd=workspace / "stages" / stage, env=role_env, expected=1)
    assert postgres_rows(postgres_dsn)[run_id]["label"] == "WRONG"
    assert {p.name: p.read_bytes() for p in (workspace / "data").iterdir()} == before
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = %s WHERE run_id = %s", (RESULT["label"], run_id))
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "down", 4)
    assert not (workspace / "data/psql_file.json").exists()
    assert (workspace / "data/psql_command.json").exists()
    move(sandbox, workspace, native_env, "down", 3)
    assert not (workspace / "data/psql_command.json").exists()
    assert (workspace / "data/postgres_inspection.json").exists()
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    project.write_text(original_project)
    move(sandbox, workspace, env, "down", 2)
    assert not (workspace / "data/postgres_inspection.json").exists()
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    move(sandbox, workspace, env, "down", 1)
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}
    assert not (workspace / "data/postgres_receipt.json").exists()
    assert json.loads((workspace / "data/tool_result.json").read_text()) == RESULT
    move(sandbox, workspace, env, "down", 0)
    status(sandbox, workspace, env, 0)
    assert list((workspace / "data").iterdir()) == []
    move(sandbox, workspace, env, "up", 5)
    second = json.loads((workspace / "data/postgres_receipt.json").read_text())
    assert second["run_id"] != run_id
    move(sandbox, workspace, env, "down", 0)
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}


@POSTGRES_ONLY
@pytest.mark.parametrize("direction", ["up", "down"])
def test_postgres_verifier_retry_preserves_committed_effect(tmp_path, cli_environment, postgres_dsn, direction):
    sandbox, workspace = materialize_pattern(tmp_path, "postgres")
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", 1 if direction == "up" else 2)
    stage = workspace / "stages/002-store-record"
    verifier = stage / f"verify-{direction}"
    original = verifier.read_text()
    verifier.write_text(original.replace('import os\n', 'import os\nraise SystemExit("injected verifier failure")\n', 1))
    target = 2 if direction == "up" else 1
    move(sandbox, workspace, env, direction, target, expected=1)
    status(sandbox, workspace, env, 1 if direction == "up" else 2, f"{direction} 2")
    committed = postgres_rows(postgres_dsn)
    assert len(committed) == (1 if direction == "up" else 0)
    verifier.write_text(original)
    (stage / direction).write_text('#!/bin/sh\nexit 99\n')
    move(sandbox, workspace, env, direction, target)
    status(sandbox, workspace, env, target)
    assert postgres_rows(postgres_dsn) == committed


@POSTGRES_ONLY
def test_postgres_commit_survives_receipt_failure_and_retry(tmp_path, cli_environment, postgres_dsn):
    sandbox, workspace = materialize_pattern(tmp_path, "postgres")
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", 1)
    # A directory at the output path forces a failure after the DB commit.
    receipt = workspace / "data/postgres_receipt.json"
    receipt.mkdir()
    move(sandbox, workspace, env, "up", 2, expected=1)
    status(sandbox, workspace, env, 1)
    committed = postgres_rows(postgres_dsn)
    assert len(committed) == 1 and list(committed.values()) == [RESULT]
    run_id = next(iter(committed))
    assert f"UUID: {run_id}" in status(sandbox, workspace, env, 1)
    # A retry must refuse to overwrite unexpected existing external data.
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = 'WRONG' WHERE run_id = %s", (run_id,))
    receipt.rmdir()
    move(sandbox, workspace, env, "up", 2, expected=1)
    assert postgres_rows(postgres_dsn)[run_id]["label"] == "WRONG"
    assert not receipt.exists()
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("UPDATE public.gallery_records SET label = %s WHERE run_id = %s", (RESULT["label"], run_id))
    move(sandbox, workspace, env, "up", 2)
    assert postgres_rows(postgres_dsn) == committed
    assert json.loads(receipt.read_text()) == {"run_id": run_id, **RESULT}
    assert f"UUID: {run_id}" in status(sandbox, workspace, env, 2)
    # Simulate an unavailable relation on down: receipt must survive failed SQL.
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("ALTER TABLE public.gallery_records RENAME TO saved_records")
    move(sandbox, workspace, env, "down", 1, expected=1)
    assert receipt.exists()
    with psycopg.connect(postgres_dsn) as connection:
        connection.execute("ALTER TABLE public.saved_records RENAME TO gallery_records")
    move(sandbox, workspace, env, "down", 1)
    assert postgres_rows(postgres_dsn) == {}
    assert not receipt.exists()


@POSTGRES_ONLY
@pytest.mark.parametrize("target", [4, 5])
def test_psql_stages_stop_on_sql_error_and_retry(tmp_path, cli_environment, postgres_dsn, target):
    sandbox, workspace = materialize_pattern(tmp_path, "postgres")
    env = dict(cli_environment, GALLERY_POSTGRES_DSN=postgres_dsn)
    prepare_postgres(workspace, postgres_dsn)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", target - 1)
    stage = workspace / "stages" / ("004-psql-command" if target == 4 else "005-psql-file")
    sql_source = stage / ("up" if target == 4 else "inspect.sql")
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
    artifact = workspace / "data" / ("psql_command.json" if target == 4 else "psql_file.json")
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


@POSTGRES_ONLY
def test_postgres_unaccepted_commit_is_abandoned_with_run_owned_cleanup(tmp_path, cli_environment, postgres_dsn):
    sandbox, workspace = materialize_pattern(tmp_path, "postgres")
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
    move(sandbox, workspace, env, "up", 1)
    checkpoint = status(sandbox, workspace, env, 1)
    run_id = checkpoint.split("UUID: ", 1)[1].splitlines()[0]
    receipt = workspace / "data/postgres_receipt.json"
    receipt.mkdir()
    move(sandbox, workspace, env, "up", 2, expected=1)
    assert status(sandbox, workspace, env, 1) == checkpoint
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated, run_id: RESULT}
    # Normal traversal cannot reverse the unaccepted stage. A sentinel proves it
    # is not executed; cleanup below is explicit user-land SQL, not a role call.
    (workspace / "stages/002-store-record/down").write_text("#!/bin/sh\nexit 99\n")
    run(
        ["psql", "-X", "--no-password", postgres_dsn, "--set=ON_ERROR_STOP=1", f"--set=run_id={run_id}"],
        cwd=sandbox, env=env,
        input_text="BEGIN;\nDELETE FROM public.gallery_records WHERE run_id = :'run_id'::uuid;\nCOMMIT;\n",
    )
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}
    assert status(sandbox, workspace, env, 1) == checkpoint
    receipt.rmdir()  # Only the artificial local blocker is removed here.
    move(sandbox, workspace, env, "down", 0)
    status(sandbox, workspace, env, 0)
    assert list((workspace / "data").iterdir()) == []
    assert postgres_rows(postgres_dsn) == {unrelated_id: unrelated}
