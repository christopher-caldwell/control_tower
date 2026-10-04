"""Optional gallery selection and isolated execution environments."""
import os
import shutil

import pytest

from common import REPOSITORY


def pytest_addoption(parser):
    parser.addoption("--family", action="append", choices=("simple", "multi_language", "py_capsule"),
                     help="run only this family; repeat to select several")
    parser.addoption("--postgres", action="store_true", help="also start disposable PostgreSQL clusters")


def pytest_configure(config):
    for family in ("simple", "multi_language", "py_capsule"):
        config.addinivalue_line("markers", f"family_{family}: {family} workflows")
    config.addinivalue_line("markers", "postgres: requires explicit --postgres")
    if config.getoption("--postgres"):
        missing = [tool for tool in ("initdb", "pg_ctl", "psql") if not shutil.which(tool)]
        if missing:
            raise pytest.UsageError("--postgres requires tools on PATH: " + ", ".join(missing))


def pytest_collection_modifyitems(config, items):
    families = config.getoption("--family")
    selected, deselected = [], []
    for item in items:
        if families and not any(item.get_closest_marker("family_" + family) for family in families):
            deselected.append(item)
            continue
        if item.get_closest_marker("postgres") and not config.getoption("--postgres"):
            item.add_marker(pytest.mark.skip(reason="opt in with ./examples/test --postgres"))
        selected.append(item)
    items[:] = selected
    config.hook.pytest_deselected(items=deselected)


@pytest.fixture
def cli_environment():
    for executable in ("control-tower", "control-tower-db"):
        assert os.access(REPOSITORY / "target/debug" / executable, os.X_OK), "run cargo build --locked --workspace first"
    env = dict(os.environ)
    for variable in (
        "PYTHONPATH", "UV_PROJECT", "UV_PROJECT_ENVIRONMENT", "UV_NO_SYNC", "NODE_PATH",
        "VIRTUAL_ENV", "UV_LOCKED", "UV_FROZEN", "GALLERY_POSTGRES_DSN",
    ):
        env.pop(variable, None)
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    env["PATH"] = str(REPOSITORY / "target/debug") + os.pathsep + env["PATH"]
    return env
