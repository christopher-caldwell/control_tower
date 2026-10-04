# Operate an existing workspace

Use this guide for an already-authored, prepared workspace. Read `control-tower guide workspace_contract` for file and process details, and `control-tower guide recover_workspace` when a movement fails or remains pending.

## CLI

The workbench commands are:

```text
control-tower status --workspace PATH
control-tower up --workspace PATH --stage NUMBER
control-tower down --workspace PATH --stage NUMBER
```

`status` reads the saved completed position, UUID, pending verification, and discovered stage count. It does not run scripts or inspect application-specific external state.

`up` applies each needed stage through the requested stage number in ascending numeric order. `down` reverses stages toward the target in descending numeric order. A number identifies a stage; it is not a count. Gaps are allowed. Stage 0 is the baseline. A settled target is a no-op and does not recheck the fixture.

The workspace must already have its prepared database. Database setup stays explicit and separate:

```text
control-tower-db bootstrap-local PATH
control-tower-db migrate-local PATH
control-tower-db verify-local PATH
```

`control-tower validate` checks the workspace in the current directory, so change into the workspace before running it. It reads the same stage and status information without executing roles or changing checkpoint state.

## Browser UI

Run `control-tower ui` from a Project directory that contains `workspaces/`. The browser host lists directory names directly under `workspaces/`; it does not search for projects or traverse example metadata. It prints a local URL for you to open manually.

Selecting a workspace loads that workspace's stages and current status. The UI separates selecting a stage for inspection from choosing a movement. Use its existing movement controls to submit one application-supplied transition at a time. The selected workspace must already be prepared. The UI does not add a new workspace format or role behavior.
