"""Copies complete examples outside the repository and walks their real stages."""
import json
from pathlib import Path
import sqlite3

import pytest

from common import (
    RESULT, RECORD, NODE_INSPECTION, PYTHON_INSPECTION, assert_node_owner,
    initialize, lock_contents, materialize_workspace, prepare_workspace, move, native_environment,
    python_environment, require_tools, run, status,
)

WORKSPACES = [
    pytest.param("simple", "uuid-file", 3, marks=pytest.mark.family_simple),
    pytest.param("simple", "generated-id", 2, marks=pytest.mark.family_simple),
    pytest.param("simple", "python-dependencies", 2, marks=pytest.mark.family_simple),
    pytest.param("simple", "node-dependencies", 2, marks=pytest.mark.family_simple),
    pytest.param("simple", "python-isolated-stage", 3, marks=pytest.mark.family_simple),
    pytest.param("multi_language", "shell-python-node", 3, marks=pytest.mark.family_multi_language),
    pytest.param("multi_language", "go-rust", 2, marks=pytest.mark.family_multi_language),
]


def assert_artifacts(workspace, name, completed):
    data = workspace / "data"
    paths = {path.name for path in data.iterdir()} if data.exists() else set()
    if name == "uuid-file":
        if not completed:
            assert not paths
        else:
            assert len(paths) == 1
            assert next(data.iterdir()).read_text() == {1: "", 2: "hello", 3: "hello to you"}[completed]
        return
    if name == "generated-id":
        # The application DB intentionally survives reversal; only its row/handoff reverse.
        if completed:
            assert paths == {"application.sqlite3", "record.json"}
            identifier = json.loads((data / "record.json").read_text())["id"]
            assert type(identifier) is int and identifier > 0
        else:
            assert paths == {"application.sqlite3"}
        with sqlite3.connect((data / "application.sqlite3").resolve().as_uri() + "?mode=ro", uri=True) as db:
            rows = db.execute("SELECT id, value FROM fixture").fetchall()
        assert rows == ([(identifier, "initial" if completed == 1 else "changed")] if completed else [])
        return
    expected = {}
    if name == "go-rust":
        if completed >= 1:
            expected["number.txt"] = "21\n"
        if completed >= 2:
            expected["doubled.txt"] = "42\n"
    else:
        if completed >= 1:
            expected["record.json"] = RECORD
        if completed >= 2:
            if name == "node-dependencies":
                expected["node_inspection.json"] = NODE_INSPECTION
            else:
                expected.update({"normalized.json": RESULT, "inspection.json": PYTHON_INSPECTION})
        if completed >= 3:
            expected["validated.json" if name == "python-isolated-stage" else "node_inspection.json"] = (
                RESULT if name == "python-isolated-stage" else NODE_INSPECTION
            )
    assert paths == set(expected)
    for filename, value in expected.items():
        content = (data / filename).read_text()
        assert (content if isinstance(value, str) else json.loads(content)) == value


@pytest.mark.parametrize(("family", "name", "target"), WORKSPACES)
def test_copied_example_workspace_traversal(tmp_path, cli_environment, family, name, target):
    if name == "generated-id":
        require_tools("python3")
    env = cli_environment
    sandbox, workspace = materialize_workspace(tmp_path, family, name)
    prepare_workspace(sandbox, workspace, cli_environment)
    locks = lock_contents(sandbox)
    initialize(sandbox, workspace, env)
    status(sandbox, workspace, env, 0)
    first_uuid = None
    for stage in range(1, target + 1):
        move(sandbox, workspace, env, "up", stage)
        checkpoint = status(sandbox, workspace, env, stage)
        run_id = checkpoint.split("UUID: ", 1)[1].splitlines()[0]
        first_uuid = first_uuid or run_id
        assert run_id == first_uuid
        assert_artifacts(workspace, name, stage)
    before = {p.name: p.stat().st_mtime_ns for p in (workspace / "data").iterdir()}
    move(sandbox, workspace, env, "up", target)
    assert before == {p.name: p.stat().st_mtime_ns for p in (workspace / "data").iterdir()}
    if name in ("python-dependencies", "python-isolated-stage", "shell-python-node"):
        info = python_environment(workspace / "stages/002-normalize", env)
        assert Path(info["prefix"]).resolve() == (sandbox / ".venv").resolve()
        assert "python-dateutil" in info["packages"]
        assert "example-tool" not in info["packages"]
        assert "jsonschema" not in info["packages"]
    if name == "python-isolated-stage":
        stage = workspace / "stages/003-inspect"
        info = python_environment(stage, env)
        assert Path(info["prefix"]).resolve() == (stage / ".venv").resolve()
        assert "jsonschema" in info["packages"]
        assert "python-dateutil" not in info["packages"]
        assert "example-tool" not in info["packages"]
        unavailable = run([str(sandbox / ".venv/bin/python"), "-c", "import jsonschema"], cwd=workspace, env=env, expected=1)
        assert "ModuleNotFoundError" in unavailable.stderr
    if name in ("node-dependencies", "shell-python-node"):
        assert_node_owner(workspace / "stages" / ("002-transform" if name == "node-dependencies" else "003-node-inspect"), sandbox, env)
    for stage in range(target - 1, -1, -1):
        move(sandbox, workspace, env, "down", stage)
        status(sandbox, workspace, env, stage)
        assert_artifacts(workspace, name, stage)
    move(sandbox, workspace, env, "up", target)
    checkpoint = status(sandbox, workspace, env, target)
    assert first_uuid != checkpoint.split("UUID: ", 1)[1].splitlines()[0]
    assert_artifacts(workspace, name, target)
    move(sandbox, workspace, env, "down", 0)
    status(sandbox, workspace, env, 0)
    assert_artifacts(workspace, name, 0)
    assert lock_contents(sandbox) == locks


@pytest.mark.family_simple
def test_isolated_stage_uses_schema_and_rejects_stale_lock(tmp_path, cli_environment):
    env = cli_environment
    sandbox, workspace = materialize_workspace(tmp_path, "simple", "python-isolated-stage")
    prepare_workspace(sandbox, workspace, cli_environment)
    initialize(sandbox, workspace, env)
    move(sandbox, workspace, env, "up", 2)
    normalized = workspace / "data/normalized.json"
    original = normalized.read_text()
    normalized.write_text('{"record_id": 123, "label": "EXAMPLE RECORD"}')
    failed = move(sandbox, workspace, env, "up", 3, expected=1)
    assert "ValidationError" in failed.stdout + failed.stderr
    status(sandbox, workspace, env, 2)
    assert not (workspace / "data/validated.json").exists()
    normalized.write_text(original)
    move(sandbox, workspace, env, "up", 3)
    move(sandbox, workspace, env, "down", 2)
    stage = workspace / "stages/003-inspect"
    project = stage / "pyproject.toml"
    project.write_text(project.read_text().replace('requires-python = ">=3.11"', 'requires-python = ">=3.12"'))
    lock = (stage / "uv.lock").read_bytes()
    locked_env = dict(env, UV_LOCKED="1", UV_OFFLINE="1")
    failed = run(["uv", "sync", "--locked", "--project", str(stage)], cwd=sandbox, env=locked_env, expected=1)
    assert "lockfile" in failed.stdout + failed.stderr
    move(sandbox, workspace, locked_env, "up", 3, expected=1)
    assert (stage / "uv.lock").read_bytes() == lock
    status(sandbox, workspace, env, 2)


@pytest.mark.family_multi_language
def test_shell_and_node_are_independent_of_python(tmp_path, cli_environment):
    env = cli_environment
    sandbox, workspace = materialize_workspace(tmp_path, "multi_language", "shell-python-node")
    prepare_workspace(sandbox, workspace, cli_environment)
    initialize(sandbox, workspace, env)
    native = native_environment(sandbox, env, "node", "mkdir", "cat", "rm")
    project = sandbox / "pyproject.toml"
    original = project.read_text()
    project.write_text("[invalid metadata\n")
    move(sandbox, workspace, native, "up", 1)
    project.write_text(original)
    move(sandbox, workspace, env, "up", 2)
    project.write_text("[invalid metadata\n")
    move(sandbox, workspace, native, "up", 3)
    assert_artifacts(workspace, "shell-python-node", 3)
    move(sandbox, workspace, native, "down", 2)
    project.write_text(original)
    move(sandbox, workspace, env, "down", 1)
    project.write_text("[invalid metadata\n")
    move(sandbox, workspace, native, "down", 0)
    status(sandbox, workspace, native, 0)
