# Operate an existing workflow

Use this guide for an already-authored, prepared workflow. Run commands from the Workspace root: the current directory must contain a valid `control-tower.toml` with a nonempty `[workspace].label` and a `workflows/` inventory. Read `control-tower guide workflow_contract` for file and process details, and `control-tower guide recover_workflow` when a movement fails or remains pending.

## CLI

Every workflow-scoped command requires an explicit `--workflow` path, resolved from the Workspace root:

```text
control-tower validate --workflow workflows/NAME
control-tower status --workflow workflows/NAME
control-tower up --workflow workflows/NAME --stage NUMBER
control-tower down --workflow workflows/NAME --stage NUMBER
```

`status` reads the saved completed position, UUID, pending verification, and discovered stage count. It does not run scripts or inspect application-specific external state.

`up` applies each needed stage through the requested stage number in ascending numeric order. `down` reverses stages toward the target in descending numeric order. A number identifies a stage; it is not a count. Gaps are allowed. Stage 0 is the baseline. A settled target is a no-op and does not recheck the fixture.

The workflow must already have its prepared database. Database setup stays explicit and separate:

```text
control-tower db bootstrap-local --workflow workflows/NAME
control-tower db migrate-local --workflow workflows/NAME
control-tower db verify-local --workflow workflows/NAME
```

Successful database commands print a short confirmation. `validate` checks the selected workflow, Workspace configuration and inventory, stage layout, saved state, and Workspace `.env` syntax. It does not execute roles or probe application/runtime prerequisites.

The Workspace-root `.env` is the only automatic dotenv source. Missing is valid; malformed or unreadable blocks validation and movement. Values provide defaults to stage executables, inherited shell values override them, and Control Tower's `CONTROL_TOWER_*` variables override both.

## Results and recovery

Control Tower returns 0 for success or a valid no-op, 1 for operational or mutation failure (including a verifier that cannot start or ends without a normal exit status), 2 for invalid CLI usage, and 3 only when a verifier returns a nonzero exit status and rejects the transition. Child-process exit status is reported separately and is not remapped as Control Tower's exit code.

Every movement ends with a compact summary of the requested target and resulting Control Tower position. When known, it includes pending verification, failed role, and child exit status. It reports Control Tower-observed facts only; it does not parse authored PASS/FAIL output or infer application assertions.

## Browser UI

Run `control-tower ui` from the Workspace root. It uses the configured label and direct workflow inventory and fails when required Workspace configuration or `workflows/` is missing. The browser host lists directory names directly under `workflows/`; it does not search for workspaces or traverse example metadata. It prints a local URL for you to open manually.

Selecting a workflow loads that workflow's stages and current status. The UI separates selecting a stage for inspection from choosing a movement. Selection is primarily inspection and does not run anything. Use **Run next** for the next stage, **Run to Stage X** to run through a selected farther future stage, or **Run all** to run through the final stage. Each submits one movement request, and the same Application movement engine used by `up` validates the target, walks intermediate stages, verifies them, and stops on failure. Before each movement, the UI reads the Workspace-root `.env` for stage-executable defaults using the same precedence as the CLI; a malformed or unreadable file prevents the movement from starting. Pending-verification retry and backout controls are separate. The selected workflow must already be prepared. The UI does not add a new workflow format or role behavior.
