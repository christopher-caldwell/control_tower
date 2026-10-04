# Edit an existing workspace

Use this guide when changing a workspace that already exists. For a new workspace, use `control-tower guide create_workspace`.

1. Work from the intended workspace directory and run `control-tower validate` to confirm its stages and saved checkpoint can be loaded. If it fails, keep the reported load/status problem visible while you inspect the workspace and database setup.
2. Run `control-tower status --workspace PATH` to see the recorded completed position, run UUID, pending verification (if any), and discovered stage count. This is saved metadata, not a fresh assertion about external application state.
3. Inspect the relevant stage files and any author-owned data or external effects before changing a pending or previously executed role. A successful or failed process does not guarantee that external state matches the checkpoint.
4. Make the smallest changes needed to existing stage files. Preserve the exact role names and matching verifier relationships in `control-tower guide workspace_contract`. Add or remove stage structure only after considering the stored position; stage-directory changes during a stored run are not reconciled automatically.
5. From inside the workspace directory, run `control-tower validate` again before considering the edit ready. It verifies that the current layout and saved state load through the existing status path. It does not execute a role or validate script logic, permissions at launch time, external effects, or application-specific correctness.

Do not delete or recreate `.control_tower/state.sqlite3` as a shortcut for reconciling scripts with external state. Check `control-tower guide recover_workspace` before retrying or reversing a failed or pending movement.
