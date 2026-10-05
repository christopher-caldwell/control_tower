# Workflow and executable contract

This guide describes the current Control Tower implementation. It does not define a workflow DSL or a protocol for values produced by scripts.

## Layout and discovery

```text
workflow/
  stages/
    001-create-fixture/
      up
      down
      verify-up       # optional
      verify-down     # optional
  .control_tower/
    state.sqlite3
```

Each immediate child directory under `stages/` is parsed as a stage. The part before the first hyphen (or the full name when there is no hyphen) must contain only ASCII decimal digits and parse as a positive `u32`; the optional text after the first hyphen is used as its label. Numeric stage identities must be unique, but need not be contiguous. A non-directory entry under `stages/` is ignored. At least one stage is required for a workflow to load.

The only recognized role filenames are `up`, `down`, `verify-up`, and `verify-down`. Each stage needs at least one mutation (`up` or `down`). A verifier requires its matching mutation. Either verifier may be absent. Role paths must be regular files; execute permission is needed when a role is launched, not during discovery or status.

The workflow database must already exist and have a supported schema. Setup is explicit and separate: `control-tower db bootstrap-local PATH`, `migrate-local PATH`, and `verify-local PATH`. `control-tower validate` does not run these operations, create storage, or repair state.

## Running roles

Control Tower directly launches a role file without adding arguments or choosing an interpreter. Provide a valid shebang, install the interpreter and dependencies yourself, and make the file executable. Each role's working directory is its own stage directory. Use `CONTROL_TOWER_WORKFLOW` for workflow-wide paths. Roles receive no interactive stdin.

The runner inherits the launching process environment and provides these additional variables:

| Variable | Meaning |
| --- | --- |
| `CONTROL_TOWER_WORKFLOW` | Canonical absolute workflow path. |
| `CONTROL_TOWER_UUID` | One Control Tower-generated UUID for the active run, shared across its stages and directions. |
| `CONTROL_TOWER_STAGE` | Numeric stage identity, without filename padding. |
| `CONTROL_TOWER_DIRECTION` | `up` or `down`. A verifier receives the direction it verifies. |
| `CONTROL_TOWER_ROLE` | `up`, `down`, `verify-up`, or `verify-down`. |

`up` establishes the stage's forward state; `down` is the author's operation for establishing the prior state. A `verify-up` or `verify-down` checks the corresponding mutation and succeeds by exiting 0. Control Tower treats a nonzero exit as failure. It does not guarantee that `down` undoes `up`.

The UUID is a run token, not a generated-output channel. Control Tower does not parse stdout or carry script-produced values into later stages. Scripts that share additional values must manage their own workflow files or other storage.

## What validation covers

`control-tower validate` takes the current working directory as the workflow. It uses the existing Workbench status path to discover stages and read/check saved checkpoint state. It does not run `up`, `down`, `verify-up`, or `verify-down`, and it does not bootstrap, migrate, repair, or write checkpoint state.

Validation does not check whether a role is executable, whether its shebang or dependencies work, whether script logic is correct or safe, or whether external application state matches the saved checkpoint. Those remain author-owned responsibilities.
