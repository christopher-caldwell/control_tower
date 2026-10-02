# Simple example family

Focused workspaces teach normal Control Tower stages, explicit application-owned
data, dependency ownership, and recovery. Each workspace is independent: copy its
directory, prepare its own dependencies, and run it with the ordinary CLIs.

| Workspace | Additional prerequisites | Lesson |
| --- | --- | --- |
| [uuid-file](workspaces/uuid-file/README.md) | Unix shell | Smallest example: one runner UUID, verification, and reversal. |
| [generated-id](workspaces/generated-id/README.md) | Python 3 with standard-library SQLite | Explicit handoff of an application-generated identifier. |
| [python-dependencies](workspaces/python-dependencies/README.md) | uv / Python 3.12 | Ordinary workspace-owned Python dependencies. |
| [node-dependencies](workspaces/node-dependencies/README.md) | Node / npm | Ordinary workspace-owned Node dependencies. |
| [python-isolated-stage](workspaces/python-isolated-stage/README.md) | uv / Python 3.12 | A closer Python project supplies a real exceptional-stage dependency. |
| [postgres](workspaces/postgres/README.md) | uv / Python 3.12 and PostgreSQL | Run-owned external effects, inspection, reversal, retry, and abandonment. |

Every dependency project lives within its workspace. The PostgreSQL workspace uses
ordinary Python/psycopg and stays focused on effect ownership. For reusable typed
capabilities, see the separate [PyCapsule family](../py_capsule/README.md).

Run `./examples/test --family simple` from the repository root; add `--postgres`
for database integration. Return to the [gallery](../README.md).
