"""Test-only CLI helpers; no workflow depends on this module."""
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
        pytest.skip("optional workflow prerequisites unavailable: " + ", ".join(missing))


def materialize_workflow(tmp_path, family, name):
    # The portable unit is the complete example, including sibling scenarios.
    sandbox = tmp_path / "copy with spaces and 'quotes'" / family
    assert not sandbox.resolve().is_relative_to(REPOSITORY.resolve()), (
        "example copies must be outside the repository; choose an external --basetemp"
    )
    ignore = shutil.ignore_patterns(
        ".venv", "__pycache__", "*.egg-info", ".control_tower", "data", "node_modules", "target",
    )
    shutil.copytree(EXAMPLES / family, sandbox, ignore=ignore)
    return sandbox, sandbox / "workflows" / name


def family_for_workflow(workflow):
    return workflow.parents[1].name


def prepare_workflow(sandbox, workflow, env):
    """Install only the selected scenario's runtimes using documented setup."""
    name = workflow.name
    python = name in (
        "python-dependencies", "python-isolated-stage", "postgres", "shell-python-node",
        "tool-only", "node-and-tool", "full-stack",
    )
    node = name in ("node-dependencies", "shell-python-node", "node-and-tool", "full-stack")
    if python:
        require_tools("uv")
    if node:
        require_tools("node", "npm")
    if name == "generated-id":
        require_tools("python3")
    if family_for_workflow(workflow) == "common_patterns":
        require_tools("node" if name == "node" else "python3")
    if name == "go-rust":
        require_tools("go", "cargo", "rustc")
    locks = lock_contents(sandbox)
    if python:
        run(["uv", "sync", "--locked", "--project", str(sandbox)], cwd=sandbox, env=env)
        if name == "python-isolated-stage":
            run(["uv", "sync", "--locked", "--project", str(workflow / "stages/003-inspect")],
                cwd=sandbox, env=env)
        if sandbox.name == "py_capsule":
            run(["uv", "sync", "--locked", "--project", str(sandbox / "tools/example_tool")],
                cwd=sandbox, env=env)
    if node:
        run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"], cwd=sandbox, env=env)
    assert lock_contents(sandbox) == locks


def initialize(sandbox, workflow, env):
    for operation in ("bootstrap-local", "migrate-local", "verify-local"):
        run(
            ["control-tower", "db", operation, "--workflow", str(workflow)],
            cwd=sandbox, env=env,
        )


def move(sandbox, workflow, env, direction, stage, *, expected=0):
    return run(
        ["control-tower", direction, "--workflow", str(workflow), "--stage", str(stage)],
        cwd=sandbox, env=env, expected=expected,
    )


def status(sandbox, workflow, env, completed, pending=None):
    text = run(["control-tower", "status", "--workflow", str(workflow)], cwd=sandbox, env=env).stdout
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
