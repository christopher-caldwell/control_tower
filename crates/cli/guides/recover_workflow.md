# Recover a failed, pending, or uncertain workflow

Control Tower records stage position, but it cannot make a workflow's scripts and external systems transactional. Inspect author-owned effects before deciding to retry or reverse.

Run commands from the Workspace root and select the workflow explicitly. Start with `control-tower status --workflow PATH` to read the checkpoint and `control-tower validate --workflow PATH` to confirm Workspace configuration, stages, dotenv syntax, and saved state still load. These commands inspect metadata; they do not establish that application data matches it.

Exit status 3 means a verifier ran and rejected the transition; inspect author-owned effects before retrying that check or reversing. Exit status 1 means an operational/mutation failure or that a verifier could not start. Exit status 2 indicates invalid CLI usage. A child's own exit status is shown in the movement result and is distinct from Control Tower's exit status. Summaries report the requested target, resulting position, pending verification, and failed role/child status when known. They do not interpret authored output or assertions.

## Pending verification

A pending transition means its mutation returned success, but its verifier has not been accepted. When the verifier fails:

- Repeating the same direction toward a reachable target retries only the matching verifier first; it does not replay the mutation.
- Requesting the opposite direction runs that stage's opposite mutation and its optional verifier. `down` is authored behavior, not an automatic rollback.
- A farther target first resolves the active transition and can then continue through other stages.

The CLI prints commands for retrying the verifier and, when the opposite mutation exists, reversing the active stage. Read the role definitions and inspect effects before using either command. Verifiers are optional and discovered again on each invocation; changing or removing a pending verifier changes what the next movement does.

## Mutation or verifier process failure

A nonzero `up` or `down` stops before its verifier and later stages. The script may have made partial external changes even though the checkpoint did not accept the stage. A failed reverse mutation can leave an earlier pending checkpoint recorded; that checkpoint does not prove what happened externally.

Inspect the role, its output, workflow data, and relevant external systems before retrying. There is no automatic rollback or dedicated recovery movement. A retry can repeat partial effects, including effects from a failed first `up` that left baseline plus a retained UUID.

## Checkpoint save failure or interruption

If saving state fails, the CLI distinguishes the last confirmed checkpoint from the attempted update. A successful role may have changed external state even when the corresponding checkpoint write failed. Re-read status to learn what is stored, then inspect the effects; an ambiguous storage error does not establish whether an update reached the database.

After process interruption, use the saved checkpoint as recorded metadata only. Inspect the relevant role and external state before further movement. Control Tower does not provide process-tree cancellation, crash reconciliation, `reset`, `force`, `skip`, or automatic cleanup. Deleting checkpoint storage does not reverse author-owned effects.

Avoid renaming, reordering, adding, or removing stage directories during a stored run. Control Tower does not reconcile a previous position with structural edits. If existing effects cannot be safely reconciled, use the workflow author's explicit cleanup procedure before continuing.
