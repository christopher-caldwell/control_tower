# Create a Workspace

Use this guide when the intended directory is not yet a Workspace, whether it is empty or already contains project files such as `TEST_PLAN.md`. Workspace creation establishes the root configuration and the directory that holds Workflows. Workflow creation authors an individual scenario and its stages.

1. Choose the intended Workspace root and inspect its existing files. Preserve project files and any existing configuration. If it already has a valid Workspace configuration and a `workflows/` directory, continue with `control-tower guide create_workflow`. To change an existing Workflow, use `control-tower guide edit_workflow`.
2. In that root, write a parseable `control-tower.toml` with at least a nonempty string label:

   ```toml
   [workspace]
   label = "My project"
   ```

3. Create a `workflows/` directory in the same root. It may be empty until you author the first Workflow. The minimum Workspace structure is:

   ```text
   new-directory/
     control-tower.toml
     workflows/
   ```

   A Workspace root `.env` is optional. Its values supply defaults for Stage Actions; inherited shell values override them, and Control Tower's `CONTROL_TOWER_*` values override both. No `.env` or checkpoint database is needed to establish the Workspace.
4. From this Workspace root, run `control-tower guide create_workflow` and follow it to author a Workflow beneath `workflows/`. Once its stages exist, run `control-tower init --workflow workflows/NAME` to prepare its database and validate it. Read `control-tower guide operate_workflow` before running Stage Actions.

Choose the guide for your starting state:

```text
empty/new directory -> create_workspace -> create_workflow -> init --workflow ... -> operate_workflow
existing Workspace  -> create_workflow
existing Workflow   -> edit_workflow
```

`control-tower guide create_workspace` prints instructions only; it does not create or repair files. Workspace setup needs only the small configuration and directory above, which the author writes directly.
