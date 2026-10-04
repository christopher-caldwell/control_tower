# Control Tower agent guidance

Choose the guide action that matches the task, then run `control-tower guide ACTION` and follow that guide. The CLI output is the version-matched source of Control Tower guidance.

- `create_workflow` — create a new workflow and author its stages.
- `edit_workflow` — inspect and change an existing workflow.
- `workflow_contract` — check the currently implemented workflow and executable rules.
- `operate_workflow` — inspect or move through an already-authored workflow with the CLI or browser UI.
- `recover_workflow` — reason about failed, pending, interrupted, or uncertain execution.

There is no separate validation guide action. The create and edit guides include when to run `control-tower validate`.
