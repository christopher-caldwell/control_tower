# Operate an existing workflow

Use this guide for an already-authored, prepared workflow. Read `control-tower guide workflow_contract` for file and process details, and `control-tower guide recover_workflow` when a movement fails or remains pending.

## CLI

The workbench commands are:

```text
control-tower status --workflow PATH
control-tower up --workflow PATH --stage NUMBER
control-tower down --workflow PATH --stage NUMBER
```

`status` reads the saved completed position, UUID, pending verification, and discovered stage count. It does not run scripts or inspect application-specific external state.

`up` applies each needed stage through the requested stage number in ascending numeric order. `down` reverses stages toward the target in descending numeric order. A number identifies a stage; it is not a count. Gaps are allowed. Stage 0 is the baseline. A settled target is a no-op and does not recheck the fixture.

The workflow must already have its prepared database. Database setup stays explicit and separate:

```text
control-tower-db bootstrap-local PATH
control-tower-db migrate-local PATH
control-tower-db verify-local PATH
```

`control-tower validate` checks the workflow in the current directory, so change into the workflow before running it. It reads the same stage and status information without executing roles or changing checkpoint state.

## Browser UI

Run `control-tower ui` from a Workspace directory that contains `workflows/`. The browser host lists directory names directly under `workflows/`; it does not search for workspaces or traverse example metadata. It prints a local URL for you to open manually.

Selecting a workflow loads that workflow's stages and current status. The UI separates selecting a stage for inspection from choosing a movement. Selection is primarily inspection and does not run anything. Use **Run next** for the next stage, **Run to Stage X** to run through a selected farther future stage, or **Run all** to run through the final stage. Each submits one movement request, and the same Application movement engine used by `up` validates the target, walks intermediate stages, verifies them, and stops on failure. Pending-verification retry and backout controls are separate. The selected workflow must already be prepared. The UI does not add a new workflow format or role behavior.
