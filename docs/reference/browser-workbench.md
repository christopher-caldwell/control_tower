---
id: CT-REF-BROWSER-WORKBENCH
title: Browser workbench
type: reference
status: maintained
created: '2026-10-02'
updated: '2026-10-03'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../crates/cli/src/web.rs
- ../../ui/src/workbench.tsx
- ../decisions/0006-loopback-web-ui.md
- ../decisions/0007-desktop-ui-shell.md
- ../research/2026-10-02-ui-reference-review.md
---

# Browser workbench

## Build the packaged UI

The graphical workbench is a macOS-only, desktop-only browser UI served by the
ordinary Rust `control-tower` executable. The production frontend is built ahead
of time and embedded with Rust. The running host needs no Node, Vite, SSR service,
or separate frontend server.

From a Control Tower checkout, rebuild the React assets and Rust executable with:

```sh
cd ui
npm ci
npm run build
npm test
npx playwright install chromium # once, for browser layout checks
npm run test:browser
cd ..
cargo build --locked --release --workspace
```

The committed `ui/dist/` is already packaged into ordinary Rust builds. Rebuild it
after changing frontend source. The browser-layout suite checks inspector collapse,
reopening, resizing, and viewport fit at the documented minimum width using
Playwright Chromium. Node and Playwright are build/test tools; the launched UI has
no Node runtime dependency.

## Starting the UI

Run `control-tower ui` from the Project directory: the directory that contains
`workspaces/`. Before launch, every discovered `workspaces/<name>/` must have
prepared storage and a valid stage layout and checkpoint. For each workspace,
run all three explicit setup operations:

```sh
control-tower-db bootstrap-local workspaces/<name>
control-tower-db migrate-local workspaces/<name>
control-tower-db verify-local workspaces/<name>
control-tower ui
```

Use the workspace directory as the argument to each database command. Stage layout
must include numbered `stages/<number>-<name>/` directories, at least one `up` or
`down` mutation in each stage, and the matching mutation for each configured
verifier. The stored checkpoint must be consistent with the discovered stages.

Any malformed or unprepared discovered workspace rejects the full UI launch before
HTTP binding or browser opening, even when its neighbors are healthy. Startup
diagnostics distinguish storage, stage-layout, checkpoint, and workspace-entry path
failures. Storage diagnostics list the three setup operations; stage and checkpoint
diagnostics describe their own corrections. Startup never creates, bootstraps, migrates,
or repairs workspace storage, stages, or checkpoints; it does not run stage scripts or
skip malformed workspaces. Correct the reported problem and launch again.

The `workspaces/` inventory must exist and contain at least one workspace directory.
Plain files in it are ignored. Canonical workspace roots are deduplicated, so symlink
aliases share one in-process movement gate. Execute permission is checked when a role
is launched; it is not a startup validation. After a successful launch, a later status
read failure can make one workspace unavailable while its neighbors remain available.
The inventory is fixed at startup; restart Control Tower to add or remove workspaces.
The project endpoint separately fans out read-only status calls for presentation and
refresh; it is not a cross-workspace operation.

## Workspace shell and movement controls

The shell shows the launch Project, a manually collapsible workspace rail, an
ordered stage narrative, and a persistent, independently scrolling, manually
collapsible and resizable stage inspector. The supported minimum width is 1180 CSS
pixels. Narrow windows retain the three regions; there is no mobile/tablet layout.
Layout preferences use browser storage at the current loopback origin and remain
stable during navigation in a running UI. A new ephemeral port can create a
different origin after restart.

Selecting a workspace or stage only changes inspection. Stage numbers are
identities, so sparse numbers such as 10 and 200 remain those numbers. Initial
stage selection follows a pending transition, then the last accepted stage, then
the first stage. The accepted checkpoint remains separate from the selected
stage. A pending stage reports its direction and an unknown prior outcome; it is
not presented as a fresh verifier failure.

The fixed action dock consumes Application's immediate `movement_choices`, never
deriving movement targets or mutation availability from the selected inspection
stage, checkpoint, or stage definitions in React. At a settled position it names
the supplied next stage and backout target. At the final accepted position there
is no forward action; backout is offered when its mutation exists. Unavailable
movements are omitted, while missing roles remain visible in the inspector.
During a pending transition,
controls distinguish `Retry verify-up`, `Retry verify-down`, backing out a
pending-up transition, and reapplying a pending-down transition. Only a verifier
failure observed in this server process receives verifier-specific choices from
`MoveOutcome::verification_choices()`. A cold pending checkpoint says its prior
execution result is unavailable. After mutation or checkpoint-save failure, the
UI warns the user to inspect author-owned effects; the pending marker is not
treated as proof that retry or reversal is safe.

`WorkbenchStatus::movement_choices()` and `MoveOutcome::movement_choices()` are
pure, synchronous Application queries. They return mechanically available
immediate movements in up/down order, without recommending recovery or promising
that authored effects are safe. Pending continuation does not require replaying
the matching mutation; reversal requires the opposite mutation. Pending choices
resolve the active stage only, including sparse identities and baseline `0`.
`MoveOutcome::verification_choices()` uses the same derivation and adds the
stronger retry/reversal guidance earned by this invocation's verifier failure.
React owns wording and primary/secondary emphasis.

Selecting earlier or future stages remains inspection only. It never changes the
movement target. Execution outcomes, including stopped movements, arrive in the live workspace snapshot. The
last confirmed checkpoint remains separate from a save's attempted update. A
running role is an invocation observation; it does not confirm that the OS
launched a child process. Captured output appears when that role returns, not as
live byte streaming.

The inspector orders checkpoint state and observed failure, mutation, verification,
relevant role output, and technical definition details. Definitions are plain
escaped text, capped at 128 KiB per role preview, and never executed while being
viewed. A previously accepted stage without retained execution evidence is
Applied, not freshly verified. A configured role with no current result is
Not attempted, Applied with historical result unavailable, or pending with prior
outcome unavailable, according to its checkpoint. A missing optional verifier is
Not configured. File presence never represents execution success.

Observed process results distinguish in progress, success, nonzero exit and
launch failure. A verifier result is shown separately from its mutation. Output
streams remain separate. UTF-8 previews replace invalid sequences and display
control bytes visibly inside an inert `<pre>`; authenticated raw stream downloads
preserve every captured byte. Results and bytes remain available for the latest
admitted movement, with no truncation or eviction limits; a newly admitted movement
replaces the previous in-process attempt. An expired movement/result reference
returns 410. This is current-attempt evidence, not durable history.

## HTTP contract and session

The API serves JSON from the same IPv4 loopback origin as the bundled UI:

| Method and path | Purpose |
| --- | --- |
| POST /api/session | Exchange the startup token for a browser session cookie. |
| GET /api/project | Aggregate independent status summaries for the startup inventory. |
| GET /api/workspaces/{id} | Perform a fresh Application status read and build a complete snapshot. |
| GET /api/workspaces/{id}/stages/{number} | Read stage definitions through Workbench and its Application-owned read capability. |
| POST /api/workspaces/{id}/movements | Submit `{direction, target_stage, expected_checkpoint}`. |
| GET /api/workspaces/{id}/events | Receive complete `snapshot` events, beginning with current state. |
| GET /api/workspaces/{id}/outputs/{operation_id}/{result_index}/{stdout\|stderr} | Read exact retained bytes for one role result. |

`expected_checkpoint` contains `completed_stage_count`, `uuid`, and `pending` (the
pending stage index and direction), copied from the displayed Application checkpoint.
It is compared inside the movement use case after the fresh state read and before any
UUID publication, checkpoint write, or process invocation. A mismatch returns 409
`stale_checkpoint`; an overlapping request returns 409 `workspace_busy`. Both use
`{ "error": { "code": string, "message": string } }`. A valid request returns
204 after invocation finishes. A dropped POST response does not cancel execution;
the client reports delivery uncertainty and never retries automatically.

Each workspace has one watch channel carrying its full latest metadata snapshot:
workspace/stage metadata, fresh current status or explicit `unavailable`, current
Application movement choices, busy state, and the complete latest attempt with every
role result. The current checkpoint and actions always come from a fresh status read.
Attempt checkpoint fields are diagnostic only. If a read fails, the snapshot has no
current checkpoint or actions while retaining the attempt, failure, last confirmed
checkpoint, attempted-but-unconfirmed values, and role results. Each event is a full
`snapshot`; notifications may coalesce, but the latest payload includes all completed
results. Reconnecting sends current state without replaying or running commands.

Role output is fetched by movement ID and role-result index; stdout and stderr stay
separate. Captures are buffered until a role returns and retained byte-for-byte until
the next admitted movement. Starting a request that fails stale validation does not
replace the previous attempt or its output. Expired output references return 410.
Stage definition previews are escaped text capped at 128 KiB per role, with separate
read diagnostics for roles that could not be read.

The workspace GET is available for explicit inspection and integration checks. React
uses the selected workspace's SSE snapshot as its sole execution-state input and does
not reconcile GET or POST bodies into that state. Its selected rail summary and action
dock are derived from the same snapshot. Project refresh may update other workspaces.

Each launch binds 127.0.0.1 on an ephemeral port and prints the usable URL before
attempting to open it with macOS `open`. The startup credential is carried only in
the URL fragment, which browsers do not send in HTTP requests. The frontend removes
the fragment and exchanges the credential for a random, HttpOnly, SameSite=Strict,
in-memory session cookie. There is no public token/configuration endpoint. Private
API reads require that cookie. API requests with an Origin header must match the
advertised origin; POST requests require it. Host must exactly match the loopback
host and ephemeral port. CORS is not enabled.

This protects against unrelated browser origins and unauthenticated local callers
accessing checkpoint details, role output or submitting a movement. It does not
sandbox trusted stage scripts or protect against malware already running as the
same macOS user. Inspect example scripts before use.

## Process and observation limits

The terminal owns the UI host process. Closing a browser tab, refreshing, changing
workspace, or losing the initiating POST response does not itself cancel, replay,
reverse or reset a movement. Within the server process, a per-workspace admission
lock remains held until the actual blocking Workbench operation and final
observation publication finish. Reads and SSE continue while a role runs. A second
tab or reconnect receives the current busy state and full latest snapshot.

Press Ctrl-C in the launching terminal to close the listener when idle. Shutdown
closes open SSE streams as part of graceful shutdown, then exits when idle. It does
not request rollback or send a cancellation signal to an active role; a blocking
Workbench call can keep graceful shutdown waiting until that call returns. Forced
termination and interruption of an active role do not prove what external effects
occurred; inspect the workspace and durable checkpoint yourself. Concurrent direct
CLI mutation of a UI-active workspace is unsupported. Unexpected worker panics or
poisoned shared locks terminate the host with a nonzero exit; relaunch after
inspecting durable checkpoint and external effects. There is no interprocess
lock, multi-instance coordination, live byte stream, durable attempt history,
project switching, or preference migration across ephemeral-port origins.

## Design references

The [UI reference review](../research/2026-10-02-ui-reference-review.md) retains
publisher-attributed screenshot links and patterns borrowed from Dagu, Inngest,
Decagon and Playwright. Remote images are source links, not runtime assets or
screenshots newly captured by this implementation.

The [macOS delivery validation](../research/2026-10-03-macos-ui-delivery.md)
records the installed-binary walkthrough, supported Rust checks, graceful idle
shutdown check, and separate implementation screenshots at the laptop minimum
width and a larger desktop viewport.
