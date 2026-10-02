"""Test-only CLI helpers; no workspace depends on this module."""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess

import pytest

EXAMPLES = Path(__file__).resolve().parents[1]
REPOSITORY = EXAMPLES.parent
RESULT = {"record_id": "123", "label": "EXAMPLE RECORD"}
RECORD = {"id": 123, "label": "  example record  ", "created_at": "2026-01-02T03:04:05+00:00"}
CONTEXT = {
    "session": {"values": {"last_record_id": "123"}, "events": []},
    "conversation": {"metadata": {"last_tool": "normalize_record"}},
}
NODE_INSPECTION = {
    "runtime": "node", "recordId": "123", "label": "EXAMPLE RECORD", "createdDate": "2026-01-02",
}
PYTHON_INSPECTION = {"runtime": "python", **RESULT, "created_date": "2026-01-02"}


def run(command, *, cwd, env, expected=0, input_text=None):
    result = subprocess.run(
        command, cwd=cwd, env=env, capture_output=True, text=True, timeout=180, input=input_text
    )
    assert result.returncode == expected, result.stdout + result.stderr
    return result


def require_tools(*executables):
    missing = [executable for executable in executables if not shutil.which(executable)]
    if missing:
        pytest.skip("optional workspace prerequisites unavailable: " + ", ".join(missing))


def materialize_workspace(tmp_path, family, name, env):
    # This location has no repository ancestors or category project for ordinary
    # workspaces. Copying that one directory must provide everything scenario-local.
    sandbox = tmp_path / "copy with spaces and 'quotes'"
    ignore = shutil.ignore_patterns(
        ".venv", "__pycache__", "*.egg-info", ".control_tower", "data", "node_modules", "target",
    )
    if family == "py_capsule":
        require_tools("uv")
        sandbox = sandbox / "py_capsule"
        sandbox.mkdir(parents=True)
        shared = EXAMPLES / family
        shutil.copytree(shared / "tools/example_tool", sandbox / "tools/example_tool", ignore=ignore)
        for filename in ("pyproject.toml", "uv.lock", ".python-version", "package.json", "package-lock.json"):
            shutil.copy2(shared / filename, sandbox / filename)
        workspace = sandbox / "workspaces" / name
    else:
        workspace = sandbox / name
    shutil.copytree(EXAMPLES / family / "workspaces" / name, workspace, ignore=ignore)
    if (workspace / "pyproject.toml").exists():
        require_tools("uv")
    if (workspace / "go.mod").exists():
        require_tools("go", "cargo", "rustc")
    node_owner = sandbox if family == "py_capsule" and name != "tool-only" else workspace
    if (node_owner / "package.json").exists():
        require_tools("node", "npm")
        run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"], cwd=node_owner, env=env)
    return sandbox, workspace


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


def lock_contents(root):
    return {path: path.read_bytes() for path in root.rglob("*")
            if path.name in ("uv.lock", "package-lock.json", "Cargo.lock", "go.sum")
            and not any(part in (".venv", "node_modules", "target") for part in path.relative_to(root).parts)}


def python_environment(project, env):
    return json.loads(run(
        ["uv", "run", "python", "-c",
         "import sys, json; from importlib.metadata import distributions; "
         "print(json.dumps({'prefix': sys.prefix, 'packages': sorted(d.metadata['Name'] for d in distributions())}))"],
        cwd=project, env=env,
    ).stdout)


def assert_node_owner(stage, owner, env):
    module = run(["node", "-p", 'require.resolve("dayjs")'], cwd=stage, env=env).stdout.strip()
    assert Path(module).is_relative_to(owner / "node_modules/dayjs")


def native_environment(sandbox, env, *executables):
    native_bin = sandbox / "native-bin"
    native_bin.mkdir(exist_ok=True)
    for executable in ("control-tower", *executables):
        target = shutil.which(executable, path=env["PATH"])
        assert target, f"required native executable missing: {executable}"
        if not (native_bin / executable).exists():
            os.symlink(target, native_bin / executable)
    return dict(env, PATH=str(native_bin))
