# Control Tower agent guidance

Agents use the CLI as the primary interface; the browser UI is primarily for human operation. The product model is Workspace → Workflow → Stage: a Workspace contains Workflows, and each Workflow is an ordered scenario of meaningful state transitions. Before creating a Workflow, briefly identify the proposed stage boundaries in your response or working notes, then continue authoring without waiting for human approval. Run commands from the Workspace root, select workflows explicitly with `--workflow`, then follow `control-tower guide ACTION`. This CLI output is the version-matched source of Control Tower guidance.

- `create_workflow` — create a new workflow and author its stages.
- `edit_workflow` — inspect and change an existing workflow.
- `workflow_contract` — check the currently implemented workflow and executable rules.
- `operate_workflow` — inspect or move through an already-authored workflow with the CLI.
- `recover_workflow` — reason about failed, pending, interrupted, or uncertain execution.

There is no separate validation guide action. The create and edit guides include when to run `control-tower validate`.
