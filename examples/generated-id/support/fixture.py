"""Author-owned SQLite fixture and JSON ID handoff; Python standard library only."""

from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import sys


DATA = Path(os.environ["CONTROL_TOWER_WORKSPACE"]) / "data"
DATABASE = DATA / "application.sqlite3"
HANDOFF = DATA / "record.json"
INITIAL = "initial"
CHANGED = "changed"


def connect(*, readonly=True):
    # Checks cannot create a missing DB, schema, or record.
    if readonly:
        return sqlite3.connect(DATABASE.resolve().as_uri() + "?mode=ro", uri=True)
    return sqlite3.connect(DATABASE)


def record_id():
    document = json.loads(HANDOFF.read_text())
    value = document.get("id") if isinstance(document, dict) else None
    if type(value) is not int or value <= 0:
        raise ValueError("record.json must contain a positive integer id")
    return value


def create():
    if HANDOFF.exists():
        raise RuntimeError("handoff already exists; inspect the fixture before creating again")
    DATA.mkdir(exist_ok=True)
    with closing(connect(readonly=False)) as db, db:
        db.execute("CREATE TABLE IF NOT EXISTS fixture (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL)")
        if db.execute("SELECT COUNT(*) FROM fixture").fetchone()[0] != 0:
            raise RuntimeError("fixture already contains records; inspect before creating again")
        identifier = db.execute("INSERT INTO fixture(value) VALUES (?)", (INITIAL,)).lastrowid
    HANDOFF.write_text(json.dumps({"id": identifier}) + "\n")
    print(f"created record {identifier}: {INITIAL}")


def delete():
    identifier = record_id()
    with closing(connect(readonly=False)) as db, db:
        if db.execute("DELETE FROM fixture WHERE id = ?", (identifier,)).rowcount != 1:
            raise RuntimeError(f"record {identifier} is missing")
    HANDOFF.unlink()
    print(f"deleted record {identifier} and handoff")


def set_value(value):
    identifier = record_id()
    with closing(connect(readonly=False)) as db, db:
        if db.execute("UPDATE fixture SET value = ? WHERE id = ?", (value, identifier)).rowcount != 1:
            raise RuntimeError(f"record {identifier} is missing")
    print(f"record {identifier}: {value}")


def verify_value(value):
    identifier = record_id()
    with closing(connect()) as db:
        actual = db.execute("SELECT value FROM fixture WHERE id = ?", (identifier,)).fetchone()
    if actual != (value,):
        raise RuntimeError(f"record {identifier}: expected {value!r}, got {actual!r}")
    print(f"verified record {identifier}: {value}")


def verify_deleted():
    with closing(connect()) as db:
        count = db.execute("SELECT COUNT(*) FROM fixture").fetchone()[0]
    if count != 0 or HANDOFF.exists():
        raise RuntimeError("fixture records or handoff remain")
    print("verified no fixture records or handoff")


OPERATIONS = {
    "create": create,
    "delete": delete,
    "change": lambda: set_value(CHANGED),
    "restore": lambda: set_value(INITIAL),
    "verify-created": lambda: verify_value(INITIAL),
    "verify-changed": lambda: verify_value(CHANGED),
    "verify-restored": lambda: verify_value(INITIAL),
    "verify-deleted": verify_deleted,
}

if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in OPERATIONS:
        sys.exit("usage: fixture.py " + "|".join(OPERATIONS))
    try:
        OPERATIONS[sys.argv[1]]()
    except (OSError, ValueError, sqlite3.Error, RuntimeError) as error:
        print(f"fixture: {error}", file=sys.stderr)
        sys.exit(1)
