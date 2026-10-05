---
id: CT-GUIDE-NAVIGATION
title: Navigate and retry verification
type: guide
status: maintained
created: '2026-10-01'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../decisions/0002-stage-navigation-and-verification.md
- ../../crates/application/src/lib.rs
- ../../crates/cli/tests/workbench_cli.rs
- ../research/run-semantics-validation.md
---

# Navigate and retry verification

A **completed stage** is Control Tower's last accepted, recorded position. A **pending transition** records that a mutation succeeded but its verifier has not succeeded. Both facts can be true at once; the stored position is not a claim that the external system remained unchanged.

Movement shows the workflow/target, then a flushed start and captured result for each actual role before the next starts. Its final completed stage, UUID and pending check use the same meaning as `status`. Output is buffered for one role, so the start identity may be the only output during a long operation. Success of a role does not by itself confirm the following checkpoint write.

## Move to a target

With the [three-stage example](../../examples/simple/workflows/uuid-file/README.md) prepared, run these from the repository root:

```sh
./target/debug/control-tower up --workflow "$workflow" --stage 2
./target/debug/control-tower status --workflow "$workflow"
./target/debug/control-tower up --workflow "$workflow" --stage 3
./target/debug/control-tower down --workflow "$workflow" --stage 2
```

The first command applies stages 1 and 2 if starting from baseline. The last reverses only stage 3. Every transition is ordered:

```text
stage N/up   -> optional stage N/verify-up   -> accept higher position
stage N/down -> optional stage N/verify-down -> accept lower position
```

An absent verifier adds no gate. A verifier that exists must pass before the destination is accepted. The next stage does not run until this one completes.

## Understand a failure

Suppose stage 2 is complete, stage 3/up succeeds, and stage 3/verify-up fails. Status should show:

```text
Completed stage: 2 (write-hello)
UUID: <the same run UUID>
Pending verification: up 3 (add-to-you)
Discovered stages: 3
```

This is representative output, not a stable machine-readable format. The file may already contain stage 3's mutation. You have two choices:

| Intent | Request | What runs |
| --- | --- | --- |
| Check stage 3 again after fixing the check or relevant application state | `up --stage 3` | Stage 3/verify-up only. The successful up does not run again. |
| Back out stage 3 | `down --stage 2` | Stage 3/down, then its optional verify-down. |

Changing application code does not itself redo the mutation. If you need to exercise the changed mutation, back out first and then run up again.

The failed movement prints runnable retry/backout commands using the current executable and workflow, with shell quoting for spaces/apostrophes. Their targets resolve only this active stage. For sparse stages 10/200, pending 200/up suggests up 200 or down 10; pending 200/down suggests down 10 or up 200. The lower side of the first stage is 0. A reverse command is offered only when that mutation exists.

Asking for `down --stage 1` first resolves stage 3's reversal, then reverses stage 2. It does not skip the unfinished stage.

## Try a verification failure

Use a **fresh disposable copy**, not a workflow containing valuable test state. These commands start from the repository root after building the CLI:

```sh
example="$(mktemp -d)/simple"
cp -R examples/simple "$example"
workflow="$example/workflows/uuid-file"
./target/debug/control-tower db bootstrap-local "$workflow"
./target/debug/control-tower db migrate-local "$workflow"
./target/debug/control-tower db verify-local "$workflow"

check="$workflow/stages/003-add-to-you/verify-up"
cp "$check" "$workflow/verify-up.original"
printf '#!/bin/sh\nexit 23\n' > "$check"
chmod +x "$check"
```

Now deliberately run a command that fails. A nonzero exit here is the expected result, not failed setup:

```sh
./target/debug/control-tower up --workflow "$workflow" --stage 3
```

Run status separately even though the preceding command failed:

```sh
./target/debug/control-tower status --workflow "$workflow"
cat "$workflow"/data/*
printf '\n'
```

The file should contain `hello to you`, but completed position remains 2 with pending up verification for stage 3.

### Back out and exercise the mutation again

```sh
./target/debug/control-tower down --workflow "$workflow" --stage 2
cat "$workflow"/data/*
printf '\n'

cp "$workflow/verify-up.original" "$check"
chmod +x "$check"
./target/debug/control-tower up --workflow "$workflow" --stage 3
./target/debug/control-tower status --workflow "$workflow"
```

After down, the file contains `hello`. After the repaired forward run it contains `hello to you`, with no pending verification.

### Alternative: retry only the verifier

Use this **instead of** the preceding back-out block while stage 3/up is still pending:

```sh
cp "$workflow/verify-up.original" "$check"
chmod +x "$check"
./target/debug/control-tower up --workflow "$workflow" --stage 3
```

Only stage 3/verify-up should appear in this invocation's role results. There is no standalone `verify` subcommand.

Checks are optional and discovered afresh. Editing/removing a pending verifier changes what the next invocation runs; removing it can accept without a check or mutation replay. Preserve the check when you intend a verifier-only retry. An already settled target runs no roles and says so; it does not reverify external state.

When finished with either path:

```sh
./target/debug/control-tower down --workflow "$workflow" --stage 0
./target/debug/control-tower status --workflow "$workflow"
```

## Downward and reverse verification

The same rules apply in both directions. If completed stage 3/down succeeds but its verify-down fails, completed stays 3. Another downward request retries only verify-down; an upward request toward 3 runs stage 3/up and its optional verifier.

Reversing an unfinished up has a different completed baseline. If stage 2 is complete, stage 3/up is pending, and stage 3/down succeeds but verify-down fails, completed stays **2**, with pending **down 3**. Another downward request retries only verify-down. The successful down is not repeated.

The UUID stays available in all these cases, including a pending return to baseline. It clears only when the workflow successfully settles at 0. SQLite retains this bookkeeping across separate CLI processes.

## A mutation failure is not a verifier failure

If `up` or `down` itself exits nonzero, Control Tower reports the failure and stops before the verifier or later stages. It does not infer whether the script partially changed something, automatically back it out, or provide a dedicated recovery flow. A failed reverse mutation can leave the prior pending checkpoint recorded; that is not evidence about the external state.

Verifier retry/backout recipes are printed only for this invocation's failed verifier. After a mutation or save failure, inspect author-owned effects. In particular, a partially effective failed first up can leave baseline, a retained UUID and no pending check. Down 0 then runs no cleanup; retrying up can repeat effects with the same UUID.

`down` reverses accepted workflow transitions. A failed mutation may leave an external
effect while its stage remains unaccepted, so normal traversal will not invoke that
stage's `down`. Recovery belongs to the workflow that owns the effect. In the
[Postgres example](../../examples/simple/workflows/postgres/README.md#failed-stage-002-retry-or-abandon),
stage 2 can commit its run-owned row and then fail writing a local receipt: completed
position stays at stage 1 and the UUID is retained. Repair/retry validates the same
row; abandonment requires explicit UUID-scoped SQL cleanup before returning to
baseline. Neither an earlier stage's reversal nor deleting checkpoint files performs
that cleanup.

After a save failure, the movement separates the last confirmed checkpoint from the attempted update, preserves role results, and stops. A pending-save failure can retain a successful mutation with no recorded pending check; a final-save failure can retain a pending check after the verifier succeeded. Those cases have different retry effects. No automatic retry/reconciliation is provided.

A Rust-process crash or SQLite failure likewise carries no external-state reconciliation guarantee. Do not treat a saved checkpoint as a transaction around an API/database mutation. See [troubleshooting](troubleshooting.md).

## Evidence

The [Application semantics tests](../../crates/application/tests/run_semantics.rs) check ordering and stored-position timing. The [CLI integrations](../../crates/cli/tests/workbench_cli.rs) cover the example and verifier retries/reversals across real processes with SQLite. The [run-semantics validation](../research/run-semantics-validation.md) records executed tests and manual runs, with toolchain/platform limits.
