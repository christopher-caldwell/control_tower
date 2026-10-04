# Control Tower agent guidance

Choose the guide action that matches the task, then run `control-tower guide ACTION` and follow that guide. The CLI output is the version-matched source of Control Tower guidance.

- `create_workspace` — create a new workspace and author its stages.
- `edit_workspace` — inspect and change an existing workspace.
- `workspace_contract` — check the currently implemented workspace and executable rules.
- `operate_workspace` — inspect or move through an already-authored workspace with the CLI or browser UI.
- `recover_workspace` — reason about failed, pending, interrupted, or uncertain execution.

There is no separate validation guide action. The create and edit guides include when to run `control-tower validate`.
