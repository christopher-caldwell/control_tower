# Polyglot

Control Tower directly executes the roles selected by their shebangs:

- Stage 001: POSIX shell seeds JSON.
- Stage 002: ordinary Python imports `example_tool` from the shared category project
  and invokes PyCapsule; no stage-local Python configuration is present.
- Stage 003: Node imports Day.js, reads the saved record and Python result, and
  writes its own JSON inspection with a formatted UTC `createdDate`.
- Stage 004: POSIX shell checks that prior files exist and writes a marker.

Node and shell never consult the category Python project. Integration tests run their
forward/reverse roles with invalid category metadata and no uv/Python on PATH. The
final shell check demonstrates handoff; preceding verifiers check result values.

The category `package.json` declares Day.js and `package-lock.json` pins it.
The category setup runs `npm ci`, and Node resolves the normal `require("dayjs")` import
from the category `node_modules`. The stage needs no `package.json`, install command,
or Node environment setting beside its roles.

See [usage and recovery](../README.md). Finish at **stage 4**, reverse to stage 0.

## Run it

After [category setup](../README.md#setup-and-run), run from the repository root:

```sh
workspace=examples/py_capsule/polyglot
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 4
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
```

Inspect `data/` between stages; the category walkthrough shows one-stage movement.
