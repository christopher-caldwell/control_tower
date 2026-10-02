# Control Tower examples

These workspaces exercise Control Tower's user-owned executable-stage API. Each
role selects its runtime through its shebang; Control Tower runs it directly from
its stage directory. Application files and external effects belong to the roles.

| I want to… | Start here |
| --- | --- |
| Try the smallest example | [simple/uuid-file](simple/uuid-file/README.md): shell roles and one runner-generated UUID. |
| See explicit application-generated data handoff | [simple/generated-id](simple/generated-id/README.md): Python's standard-library SQLite and author-owned JSON. |
| Use PyCapsule | [py_capsule](py_capsule/README.md): shared Python dependencies, an isolated stage, polyglot roles, and optional PostgreSQL. |
| Explore advanced non-PyCapsule multi-system workflows | [complex](complex/README.md): the category for future examples. |

The [root quickstart](../README.md#try-the-three-stage-example) needs no Python or
Node packages. PyCapsule setup and dependencies stay within its category.
See [workspace authoring](../docs/guides/creating-a-workspace.md) and the
[executable contract](../docs/reference/stage-executables.md) for the common API.
