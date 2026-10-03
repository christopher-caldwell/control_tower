"""Shared typed tool, real capsule child, and runtime boundaries."""
import json
import tomllib
from pathlib import Path

import pytest

from common import (
    RESULT, RECORD, CONTEXT, materialize_workspace, prepare_workspace, initialize, move, status,
    run, lock_contents, python_environment, assert_node_owner, native_environment,
)

pytestmark = pytest.mark.family_py_capsule
PATTERNS = [("tool-only", 3), ("node-and-tool", 3)]


@pytest.mark.parametrize("project", [".", "tools/example_tool"])
def test_published_capsule_dependency_is_pinned(tmp_path, project):
    sandbox, _ = materialize_workspace(tmp_path, "py_capsule", "tool-only")
    project_root = sandbox / project
    manifest = tomllib.loads((project_root / "pyproject.toml").read_text())
    assert "capsule-runner==0.0.1" in manifest["project"]["dependencies"]
    sources = manifest.get("tool", {}).get("uv", {}).get("sources", {})
    assert "py-capsule" not in sources
    assert "capsule-runner" not in sources
    lock = tomllib.loads((project_root / "uv.lock").read_text())
    packages = {package["name"]: package for package in lock["package"]}
    assert "py-capsule" not in packages
    assert packages["capsule-runner"]["version"] == "0.0.1"
    assert packages["capsule-runner"]["source"] == {"registry": "https://pypi.org/simple"}


def assert_artifacts(workspace, name, completed):
    expected = {}
    if completed >= 1:
        expected["record.json"] = RECORD
    if completed >= 2:
        expected.update({"tool_result.json": RESULT, "runtime_context.json": CONTEXT})
    if completed >= 3:
        if name == "node-and-tool":
            expected["node_inspection.json"] = {
                "runtime": "node", "recordId": "123", "label": "EXAMPLE RECORD", "createdDate": "2026-01-02",
            }
        else:
            expected["inspection.json"] = {"result": RESULT, "last_record_id": "123", "last_tool": "normalize_record"}
    data = workspace / "data"
    assert ({path.name for path in data.iterdir()} if data.exists() else set()) == set(expected)
    for filename, value in expected.items():
        content = (data / filename).read_text()
        assert (content if isinstance(value, str) else json.loads(content)) == value


def assert_environment_ownership(sandbox, workspace, env, name):
    projects = [workspace / "stages/002-call-tool"]
    if name == "tool-only":
        projects += [workspace / "stages/001-seed", workspace / "stages/003-inspect"]
    for project in projects:
        info = python_environment(project, env)
        assert Path(info["prefix"]).resolve() == (sandbox / ".venv").resolve()
        assert "example-tool" in info["packages"]
        assert "capsule-runner" in info["packages"]
        assert "py-capsule" not in info["packages"]
        assert "python-dateutil" in info["packages"]
        assert "jsonschema" not in info["packages"]
        source = run(
            ["uv", "run", "python", "-c", "import example_tool; print(example_tool.__file__)"],
            cwd=project, env=env,
        ).stdout.strip()
        assert Path(source).resolve().is_relative_to(sandbox / "tools/example_tool/src")
    if name == "node-and-tool":
        assert_node_owner(workspace / "stages/003-node-inspect", sandbox, env)
    # Real PyCapsule child execution creates a distinct tool-owned environment.
    assert (sandbox / "tools/example_tool/.venv").is_dir()


@pytest.mark.parametrize(("name", "target"), PATTERNS)
def test_workspace_pattern_runs_through_real_control_tower(tmp_path, cli_environment, name, target):
    sandbox, workspace = materialize_workspace(tmp_path, "py_capsule", name)
    prepare_workspace(sandbox, workspace, cli_environment)
    env = cli_environment
    locks_before = lock_contents(sandbox)
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
    assert lock_contents(sandbox) == locks_before


def test_workspaces_reuse_one_copied_tool_with_separate_state(tmp_path, cli_environment):
    sandbox, tool_only = materialize_workspace(tmp_path, "py_capsule", "tool-only")
    node_and_tool = sandbox / "workspaces/node-and-tool"
    env = cli_environment
    locks = lock_contents(sandbox)
    prepare_workspace(sandbox, node_and_tool, env)
    for workspace in (tool_only, node_and_tool):
        initialize(sandbox, workspace, env)
        move(sandbox, workspace, env, "up", 3)
        assert_artifacts(workspace, workspace.name, 3)
        assert_environment_ownership(sandbox, workspace, env, workspace.name)
    saved = {p.name: p.read_bytes() for p in (node_and_tool / "data").iterdir()}
    tool_uuid = status(sandbox, tool_only, env, 3).split("UUID: ", 1)[1].splitlines()[0]
    node_uuid = status(sandbox, node_and_tool, env, 3).split("UUID: ", 1)[1].splitlines()[0]
    assert tool_uuid != node_uuid
    move(sandbox, tool_only, env, "down", 0)
    status(sandbox, tool_only, env, 0)
    assert_artifacts(tool_only, "tool-only", 0)
    status(sandbox, node_and_tool, env, 3)
    assert {p.name: p.read_bytes() for p in (node_and_tool / "data").iterdir()} == saved
    move(sandbox, node_and_tool, env, "down", 0)
    status(sandbox, node_and_tool, env, 0)
    assert_artifacts(node_and_tool, "node-and-tool", 0)
    assert lock_contents(sandbox) == locks


def test_new_ticket_needs_only_executable_python_roles(tmp_path, cli_environment):
    sandbox, selected_workspace = materialize_workspace(tmp_path, "py_capsule", "tool-only")
    prepare_workspace(sandbox, selected_workspace, cli_environment)
    workspace = sandbox / "workspaces/ticket_123"
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


def test_node_roles_work_without_a_usable_python_project(tmp_path, cli_environment):
    name = "node-and-tool"
    sandbox, workspace = materialize_workspace(tmp_path, "py_capsule", name)
    prepare_workspace(sandbox, workspace, cli_environment)
    env = cli_environment
    initialize(sandbox, workspace, env)
    # No uv/python on PATH and invalid family metadata expose accidental coupling.
    native_env = native_environment(sandbox, env, "node")
    project = sandbox / "pyproject.toml"
    original = project.read_text()
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "up", 1)
    assert_artifacts(workspace, name, 1)
    project.write_text(original)
    move(sandbox, workspace, env, "up", 2)
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "up", 3)
    assert_artifacts(workspace, name, 3)
    move(sandbox, workspace, native_env, "down", 2)
    assert_artifacts(workspace, name, 2)
    project.write_text(original)
    move(sandbox, workspace, env, "down", 1)
    project.write_text("[invalid project metadata\n")
    move(sandbox, workspace, native_env, "down", 0)
    status(sandbox, workspace, native_env, 0)
    assert_artifacts(workspace, name, 0)


@pytest.mark.parametrize("direction", ["up", "down"])
def test_verifier_retry_does_not_repeat_mutation(tmp_path, cli_environment, direction):
    name = "tool-only"
    sandbox, workspace = materialize_workspace(tmp_path, "py_capsule", name)
    prepare_workspace(sandbox, workspace, cli_environment)
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


@pytest.mark.parametrize("owner", ["family", "tool"])
def test_strict_validation_rejects_stale_dependency_lock(tmp_path, cli_environment, owner):
    name = "tool-only"
    sandbox, workspace = materialize_workspace(tmp_path, "py_capsule", name)
    prepare_workspace(sandbox, workspace, cli_environment)
    # Exercise offline stale-lock rejection in the caller and capsule child.
    env = dict(cli_environment, UV_LOCKED="1", UV_OFFLINE="1")
    initialize(sandbox, workspace, env)
    project = {
        "family": sandbox / "pyproject.toml",
        "tool": sandbox / "tools/example_tool/pyproject.toml",
    }[owner]
    if owner != "family":
        move(sandbox, workspace, env, "up", 1)
    if owner == "tool":
        project.write_text(project.read_text() + '\n[dependency-groups]\ndev = ["pytest>=8,<10"]\n')
    else:
        project.write_text(project.read_text().replace('requires-python = ">=3.11"', 'requires-python = ">=3.12"'))
    lock = project.with_name("uv.lock")
    before = lock.read_bytes()
    # Locked setup can install artifacts without caching registry metadata. Resolve
    # the changed project once, then restore its stale lock so offline rejection
    # tests lock validation rather than whether another test warmed uv's cache.
    run(["uv", "lock", "--project", str(project.parent)], cwd=sandbox, env=cli_environment)
    lock.write_bytes(before)
    if owner == "tool":
        # Refresh only the caller's lock. The child stays stale, proving validation
        # reaches PyCapsule's uv rather than failing in the calling environment.
        run(["uv", "lock", "--project", str(sandbox)], cwd=sandbox, env=cli_environment)
        run(["uv", "lock", "--check", "--project", str(sandbox)], cwd=sandbox, env=env)
    # The same locked sync used by setup must reject stale metadata too.
    rejected = run(["uv", "sync", "--locked", "--project", str(project.parent)], cwd=sandbox, env=env, expected=1)
    assert "lockfile" in rejected.stdout + rejected.stderr
    failed = move(sandbox, workspace, env, "up", {"family": 1, "tool": 2}[owner], expected=1)
    assert "lockfile" in failed.stdout + failed.stderr
    assert lock.read_bytes() == before
    assert_artifacts(workspace, name, {"family": 0, "tool": 1}[owner])
