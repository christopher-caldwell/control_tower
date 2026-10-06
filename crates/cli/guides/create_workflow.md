# Create a workflow

Use this guide for a new workflow. It assumes the current directory is already the intended Workspace root. For changes to an existing workflow, use `control-tower guide edit_workflow` instead.

The Workspace must contain a parseable `control-tower.toml` with a nonempty `[workspace].label`, plus a `workflows/` directory. Create the new workflow under `workflows/`; do not create or repair Workspace configuration as part of creating an individual workflow. A Workspace-root `.env` is optional. Its values are defaults for stage executables, shell values take precedence, and Control Tower's `CONTROL_TOWER_*` values take precedence over both.

1. Choose a new, unused workflow directory beneath `workflows/`.
2. If you need exact current filenames, numbering, environment variables, or process behavior, read `control-tower guide workflow_contract`.
3. Add a `stages/` directory and numbered stage directories. Put each role in a regular file named exactly `up`, `down`, `verify-up`, or `verify-down`; use a valid shebang and make roles executable. Each stage needs `up` or `down`. A verifier is optional and must have its matching mutation.
4. Author `up` and `down` to establish the forward and prior states your workflow needs. Make verifiers observe their result and exit 0 only when it is accepted. Control Tower does not supply application-specific operations or assertions.
5. Prepare the workflow database explicitly with `control-tower db bootstrap-local --workflow workflows/NAME`, then `control-tower db migrate-local --workflow workflows/NAME`, and `control-tower db verify-local --workflow workflows/NAME`. These are separate setup commands; ordinary workflow commands do not prepare storage.
6. From the Workspace root, run `control-tower validate --workflow workflows/NAME` before considering authoring ready. It checks that Control Tower can discover the stages and read the prepared checkpoint state. It does not run roles or assess their application-specific correctness or safety.

Workflow-scoped commands require `--workflow`; paths are resolved from the Workspace root. A valid no-op and success return 0, operational failures return 1, CLI usage errors return 2, and 3 means a verifier returned a nonzero exit status and rejected the transition. A verifier that cannot start or terminates without a normal exit status returns 1. After exit 3, inspect role output and author-owned effects before retrying the verifier or reversing the stage. The child's exit status appears in the movement summary when known and is distinct from Control Tower's exit code. See `control-tower guide recover_workflow` for recovery choices.

The first `up` movement can run every earlier unapplied stage in numeric order. A target is a stage number, not an instruction to run only that one script. Read `control-tower guide operate_workflow` before operating the workflow.
