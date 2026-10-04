# Create a workspace

Use this guide for a new workspace. For changes to an existing workspace, use `control-tower guide edit_workspace` instead.

1. Choose a new directory for the workspace.
2. If you need exact current filenames, numbering, environment variables, or process behavior, read `control-tower guide workspace_contract`.
3. Add a `stages/` directory and numbered stage directories. Put each role in a regular file named exactly `up`, `down`, `verify-up`, or `verify-down`; use a valid shebang and make roles executable. Each stage needs `up` or `down`. A verifier is optional and must have its matching mutation.
4. Author `up` and `down` to establish the forward and prior states your workflow needs. Make verifiers observe their result and exit 0 only when it is accepted. Control Tower does not supply application-specific operations or assertions.
5. Prepare the workspace database explicitly with `control-tower-db bootstrap-local PATH`, then `migrate-local PATH`, and `verify-local PATH`. Replace `PATH` with the workspace directory. These are separate setup commands; ordinary workspace commands do not prepare storage.
6. From inside the workspace directory, run `control-tower validate` before considering authoring ready. It checks that Control Tower can discover the stages and read the prepared checkpoint state. It does not run roles or assess their application-specific correctness or safety.

The first `up` movement can run every earlier unapplied stage in numeric order. A target is a stage number, not an instruction to run only that one script. Read `control-tower guide operate_workspace` before operating the workspace.
