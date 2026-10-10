"""Copies complete examples outside the repository and walks their real stages."""
import json
from pathlib import Path
import re
import sqlite3

import pytest

from common import (
    RESULT, RECORD, NODE_INSPECTION, PYTHON_INSPECTION, REPOSITORY, assert_node_owner,
    initialize, lock_contents, materialize_workflow, prepare_workflow, move, native_environment,
    python_environment, require_tools, run, status,
)

WORKFLOWS = [
    pytest.param("simple", "uuid-file", 3, marks=pytest.mark.family_simple),
    pytest.param("simple", "generated-id", 2, marks=pytest.mark.family_simple),
    pytest.param("simple", "python-dependencies", 2, marks=pytest.mark.family_simple),
    pytest.param("simple", "node-dependencies", 2, marks=pytest.mark.family_simple),
    pytest.param("simple", "python-isolated-stage", 3, marks=pytest.mark.family_simple),
    pytest.param("multi_language", "shell-python-node", 3, marks=pytest.mark.family_multi_language),
    pytest.param("multi_language", "go-rust", 2, marks=pytest.mark.family_multi_language),
    pytest.param("common_patterns", "node", 2, marks=pytest.mark.family_common_patterns),
    pytest.param("common_patterns", "python", 2, marks=pytest.mark.family_common_patterns),
]


@pytest.mark.family_simple
def test_documented_uuid_file_recovery_walkthrough(tmp_path, cli_environment):
    # Execute the guide itself so its shell commands cannot drift from the test.
    guide = (REPOSITORY / "docs/guides/verification-and-navigation.md").read_text()
    walkthrough = guide.split("## Try a verification failure\n", 1)[1].split(
        "## Downward and reverse verification\n", 1
    )[0]
    blocks = re.findall(r"^```sh\n(.*?)^```", walkthrough, flags=re.MULTILINE | re.DOTALL)
    assert blocks, "the recovery walkthrough must contain executable shell commands"
    temporary = tmp_path / "copy with spaces and 'quotes'"
    temporary.mkdir()
    env = native_environment(
        tmp_path, cli_environment, "sh", "mktemp", "cp", "chmod", "cat", "ls", "mkdir", "rm"
    )
    env["TMPDIR"] = str(temporary)
    # Python runs this harness, but is unavailable to the walkthrough and actions.
    result = run(
        ["/bin/sh", "-eu"], cwd=REPOSITORY, env=env, input_text="\n".join(blocks)
    )
    assert "Failed Stage Action: stage 3 verify-up; child exit status 23" in result.stdout
    checkpoints = re.findall(
        r"^Completed stage: (.+)\nUUID: (.+)\n"
        r"(?:Pending verification: (.+)\n)?Discovered stages: (\d+)$",
        result.stdout, flags=re.MULTILINE,
    )
    assert checkpoints, "the walkthrough must report checkpoints"
    run_id = checkpoints[0][1]
    assert run_id != "not created"
    # Each movement and its following status must report the same checkpoint.
    expected = [
        ("2 (write-hello)", run_id, "", "3"),
        ("2 (write-hello)", run_id, "up 3 (add-to-you)", "3"),
        ("3 (add-to-you)", run_id, "", "3"),
        ("baseline (0)", "not created", "", "3"),
    ]
    assert checkpoints == [checkpoint for checkpoint in expected for _ in range(2)]
    started = re.findall(r"^\[stage (\d+) ([\w-]+) \([^\n]+\)\] starting$", result.stdout, re.MULTILINE)
    assert started == [
        ("1", "up"), ("1", "verify-up"),
        ("2", "up"), ("2", "verify-up"),
        ("3", "up"), ("3", "verify-up"),
        ("3", "verify-up"),  # Corrected retry runs only the verifier.
        ("3", "down"), ("3", "verify-down"),
        ("2", "down"), ("2", "verify-down"),
        ("1", "down"), ("1", "verify-down"),
    ]
    workflow = Path(re.search(r"^Walkthrough workflow: (.+)$", result.stdout, re.MULTILINE)[1])
    assert workflow.resolve().is_relative_to(temporary.resolve())
    assert (workflow / "mutation-calls").read_text() == "3 up\n"
    assert list((workflow / "data").iterdir()) == []


def assert_artifacts(workflow, name, completed):
    data = workflow / "data"
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
    if workflow.parents[1].name == "common_patterns":
        if not completed:
            assert not paths
            return
        state = json.loads((data / "state.json").read_text())
        generated_id = state["generated"]["id"]
        assert isinstance(generated_id, str) and generated_id
        assert json.loads(state["generated"]["captured_stdout"])["id"] == generated_id
        if completed == 1:
            assert set(state) == {"generated"}
        else:
            assert set(state) == {"generated", "verified"}
            assert state["verified"] == generated_id
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


@pytest.mark.parametrize(("family", "name", "target"), WORKFLOWS)
def test_copied_example_workflow_traversal(tmp_path, cli_environment, family, name, target):
    if name == "generated-id":
        require_tools("python3")
    env = cli_environment
    sandbox, workflow = materialize_workflow(tmp_path, family, name)
    prepare_workflow(sandbox, workflow, cli_environment)
    locks = lock_contents(sandbox)
    initialize(sandbox, workflow, env)
    status(sandbox, workflow, env, 0)
    first_uuid = None
    for stage in range(1, target + 1):
        move(sandbox, workflow, env, "up", stage)
        checkpoint = status(sandbox, workflow, env, stage)
        run_id = checkpoint.split("UUID: ", 1)[1].splitlines()[0]
        first_uuid = first_uuid or run_id
        assert run_id == first_uuid
        assert_artifacts(workflow, name, stage)
    before = {p.name: p.stat().st_mtime_ns for p in (workflow / "data").iterdir()}
    move(sandbox, workflow, env, "up", target)
    assert before == {p.name: p.stat().st_mtime_ns for p in (workflow / "data").iterdir()}
    if name in ("python-dependencies", "python-isolated-stage", "shell-python-node"):
        info = python_environment(workflow / "stages/002-normalize", env)
        assert Path(info["prefix"]).resolve() == (sandbox / ".venv").resolve()
        assert "python-dateutil" in info["packages"]
        assert "example-tool" not in info["packages"]
        assert "jsonschema" not in info["packages"]
    if name == "python-isolated-stage":
        stage = workflow / "stages/003-inspect"
        info = python_environment(stage, env)
        assert Path(info["prefix"]).resolve() == (stage / ".venv").resolve()
        assert "jsonschema" in info["packages"]
        assert "python-dateutil" not in info["packages"]
        assert "example-tool" not in info["packages"]
        unavailable = run([str(sandbox / ".venv/bin/python"), "-c", "import jsonschema"], cwd=workflow, env=env, expected=1)
        assert "ModuleNotFoundError" in unavailable.stderr
    if name in ("node-dependencies", "shell-python-node"):
        assert_node_owner(workflow / "stages" / ("002-transform" if name == "node-dependencies" else "003-node-inspect"), sandbox, env)
    for stage in range(target - 1, -1, -1):
        move(sandbox, workflow, env, "down", stage)
        status(sandbox, workflow, env, stage)
        assert_artifacts(workflow, name, stage)
    move(sandbox, workflow, env, "up", target)
    checkpoint = status(sandbox, workflow, env, target)
    assert first_uuid != checkpoint.split("UUID: ", 1)[1].splitlines()[0]
    assert_artifacts(workflow, name, target)
    move(sandbox, workflow, env, "down", 0)
    status(sandbox, workflow, env, 0)
    assert_artifacts(workflow, name, 0)
    assert lock_contents(sandbox) == locks


@pytest.mark.parametrize(
    ("family", "name", "runtime"),
    [
        pytest.param("common_patterns", "node", "node", marks=pytest.mark.family_common_patterns),
        pytest.param("common_patterns", "python", "python3", marks=pytest.mark.family_common_patterns),
    ],
)
def test_common_pattern_verify_down_rejects_remaining_verified_value(
    tmp_path, cli_environment, family, name, runtime
):
    env = cli_environment
    sandbox, workflow = materialize_workflow(tmp_path, family, name)
    prepare_workflow(sandbox, workflow, env)
    initialize(sandbox, workflow, env)
    move(sandbox, workflow, env, "up", 2)

    state_file = workflow / "data/state.json"
    before = json.loads(state_file.read_text())
    assert "verified" in before

    # Simulate a broken/no-op Stage 2 reversal. Control Tower must run the
    # Stage Action verifier and reject the transition while verified remains.
    down = workflow / "stages/002-consume/down"
    no_op = "#!/usr/bin/env node\n// Intentionally leaves Stage 2 state unchanged.\n" if runtime == "node" else (
        "#!/usr/bin/env python3\n# Intentionally leaves Stage 2 state unchanged.\n"
    )
    down.write_text(no_op)
    rejected = move(sandbox, workflow, env, "down", 1, expected=3)
    output = rejected.stdout + rejected.stderr
    assert "Failed Stage Action: stage 2 verify-down" in output
    assert "child exit status 1" in output
    assert json.loads(state_file.read_text()) == before, "the verifier must observe, not repair, state"


@pytest.mark.family_common_patterns
def test_node_saved_values_require_own_properties(tmp_path, cli_environment):
    require_tools("node")
    sandbox, workflow = materialize_workflow(tmp_path, "common_patterns", "node")
    env = dict(cli_environment, CONTROL_TOWER_WORKFLOW=str(workflow))
    run([
        "node", "--input-type=module", "-e", """
        import assert from 'node:assert/strict';
        import { saveValue, loadValue } from './lib/node/index.mjs';

        await saveValue('generated', { id: 'example-id' });
        assert.deepEqual(await loadValue('generated'), { id: 'example-id' });
        for (const key of ['missing', 'toString', 'constructor']) {
            await assert.rejects(loadValue(key), { message: `Missing saved value: ${key}` });
        }
        for (const [key, value] of Object.entries({ empty: '', zero: 0, flag: false, nil: null })) {
            await saveValue(key, value);
            assert.equal(await loadValue(key), value);
        }
        await saveValue('toString', 'saved explicitly');
        assert.equal(await loadValue('toString'), 'saved explicitly');
        """,
    ], cwd=sandbox, env=env)


@pytest.mark.family_simple
def test_isolated_stage_uses_schema_and_rejects_stale_lock(tmp_path, cli_environment):
    env = cli_environment
    sandbox, workflow = materialize_workflow(tmp_path, "simple", "python-isolated-stage")
    prepare_workflow(sandbox, workflow, cli_environment)
    initialize(sandbox, workflow, env)
    move(sandbox, workflow, env, "up", 2)
    normalized = workflow / "data/normalized.json"
    original = normalized.read_text()
    normalized.write_text('{"record_id": 123, "label": "EXAMPLE RECORD"}')
    failed = move(sandbox, workflow, env, "up", 3, expected=1)
    assert "ValidationError" in failed.stdout + failed.stderr
    status(sandbox, workflow, env, 2)
    assert not (workflow / "data/validated.json").exists()
    normalized.write_text(original)
    move(sandbox, workflow, env, "up", 3)
    move(sandbox, workflow, env, "down", 2)
    stage = workflow / "stages/003-inspect"
    project = stage / "pyproject.toml"
    project.write_text(project.read_text().replace('requires-python = ">=3.11"', 'requires-python = ">=3.12"'))
    lock = (stage / "uv.lock").read_bytes()
    locked_env = dict(env, UV_LOCKED="1", UV_OFFLINE="1")
    failed = run(["uv", "sync", "--locked", "--project", str(stage)], cwd=sandbox, env=locked_env, expected=1)
    assert "lockfile" in failed.stdout + failed.stderr
    move(sandbox, workflow, locked_env, "up", 3, expected=1)
    assert (stage / "uv.lock").read_bytes() == lock
    status(sandbox, workflow, env, 2)


@pytest.mark.family_multi_language
def test_shell_and_node_are_independent_of_python(tmp_path, cli_environment):
    env = cli_environment
    sandbox, workflow = materialize_workflow(tmp_path, "multi_language", "shell-python-node")
    prepare_workflow(sandbox, workflow, cli_environment)
    initialize(sandbox, workflow, env)
    native = native_environment(sandbox, env, "node", "mkdir", "cat", "rm")
    project = sandbox / "pyproject.toml"
    original = project.read_text()
    project.write_text("[invalid metadata\n")
    move(sandbox, workflow, native, "up", 1)
    project.write_text(original)
    move(sandbox, workflow, env, "up", 2)
    project.write_text("[invalid metadata\n")
    move(sandbox, workflow, native, "up", 3)
    assert_artifacts(workflow, "shell-python-node", 3)
    move(sandbox, workflow, native, "down", 2)
    project.write_text(original)
    move(sandbox, workflow, env, "down", 1)
    project.write_text("[invalid metadata\n")
    move(sandbox, workflow, native, "down", 0)
    status(sandbox, workflow, native, 0)
