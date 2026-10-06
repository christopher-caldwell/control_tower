"""Small author-owned helpers for captured results and cross-stage JSON state."""
import json
import os
from pathlib import Path
import subprocess


def _state_file():
    workflow = os.environ.get("CONTROL_TOWER_WORKFLOW")
    if not workflow:
        raise RuntimeError("CONTROL_TOWER_WORKFLOW is required")
    return Path(workflow) / "data" / "state.json"


def save_value(key, value):
    path = _state_file()
    try:
        state = json.loads(path.read_text())
    except FileNotFoundError:
        state = {}
    state[key] = value
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(state, indent=2) + "\n")


def load_value(key):
    state = json.loads(_state_file().read_text())
    if key not in state:
        raise AssertionError(f"Missing saved value: {key}")
    return state[key]


def require_absent_value(key):
    state = json.loads(_state_file().read_text())
    if key in state:
        raise AssertionError(f"Expected saved value to be absent: {key}")


def delete_value(key):
    path = _state_file()
    state = json.loads(path.read_text())
    state.pop(key, None)
    path.write_text(json.dumps(state, indent=2) + "\n")


def capture(command, *args):
    return subprocess.run([command, *args], capture_output=True, text=True, check=False)


def require_success(result):
    if result.returncode != 0:
        raise AssertionError(f"Operation failed ({result.returncode}): {result.stderr}")
    return result


def require_equal(actual, expected, label="value"):
    if actual != expected:
        raise AssertionError(f"{label}: expected {expected!r}, got {actual!r}")
