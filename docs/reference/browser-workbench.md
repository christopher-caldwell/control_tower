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

## Build and launch

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

The directory where `control-tower ui` starts is the Project. For example, from
the repository root, launch from `examples/simple`, not from inside a workspace.
Copy a complete example project before using it as scratch space. Each workspace
has its own checkpoint database, and storage setup remains explicit:

```sh
repo="$(pwd)"
project="$(mktemp -d)/simple"
cp -R "$repo/examples/simple" "$project"
workspace="$project/workspaces/uuid-file"
"$repo/target/release/control-tower-db" bootstrap-local "$workspace"
"$repo/target/release/control-tower-db" migrate-local "$workspace"
"$repo/target/release/control-tower-db" verify-local "$workspace"
(cd "$project" && "$repo/target/release/control-tower" ui)
```

Every immediate child directory in the Project workspaces/ directory is shown as a
workspace candidate. Candidates missing stages/ or containing invalid stages stay
visible with an unavailable reason. The UI does not run stages during discovery or
status reads. Workspaces with unprepared or inaccessible storage are not treated as
baseline. The Project list is frozen at startup. Refresh rereads checkpoint status
for that list; restart Control Tower to add or remove workspaces.

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

The fixed action dock is derived from the durable checkpoint and ordered stage
list, never from the selected inspection stage. At a settled position it names
the actual next stage and the stage that would be reversed. The last accepted
stage has a backout action and no forward action. During a pending transition,
controls distinguish `Retry verify-up`, `Retry verify-down`, backing out a
pending-up transition, and reapplying a pending-down transition. Only a verifier
failure observed in this server process receives verifier-specific choices from
`MoveOutcome::verification_choices()`. A cold pending checkpoint says its prior
execution result is unavailable. After mutation or checkpoint-save failure, the
UI warns the user to inspect author-owned effects; the pending marker is not
treated as proof that retry or reversal is safe.

Selecting earlier or future stages remains inspection only. It never changes the
movement target. A stopped 2xx movement response is displayed as stopped, and the
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
preserve retained bytes exactly. Each stdout/stderr stream retains at most its
first 1 MiB and reports the original byte count and truncation. Per workspace,
captured output is capped at 8 MiB and role summaries at 128 per movement; evicted
output and omitted older summaries are labelled. A new movement replaces that
workspace's previous in-process movement record. This is bounded current/latest
observation, not durable history.

## HTTP contract and session

The API serves JSON from the same IPv4 loopback origin as the bundled UI:

| Method and path | Purpose |
| --- | --- |
| POST /api/session | Exchange the startup token for a browser session cookie. |
| GET /api/project | List startup-discovered workspaces and their independent status. |
| GET /api/workspaces/{id} | Read checkpoint identity, ordered stage summaries, busy state and current/latest process-local observation. |
| GET /api/workspaces/{id}/stages/{number} | Read one stage role definitions. |
| POST /api/workspaces/{id}/movements | Submit one `{direction, target_stage}` intent. The target is a real stage number; 0 is baseline. |
| GET /api/workspaces/{id}/events | Receive workspace-scoped SSE notifications and an initial resynchronization snapshot. |
| GET /api/workspaces/{id}/outputs/{output_id}/{stdout\|stderr} | Read one retained raw output stream for a completed role. |

### Movement and observation shapes

Movement JSON is `{ "direction": "up" | "down", "target_stage": number }`.
The target is a stage-number identity, not an array index; `0` names baseline for
downward movement. A delivered movement result is `{ "observation": ... }`:

```json
{
  "observation": {
    "workspace_id": "uuid-file",
    "operation_id": "per-movement-id",
    "server_instance_id": "per-launch-id",
    "revision": 14,
    "direction": "up",
    "target_stage": 200,
    "state": "stopped",
    "active_role": null,
    "role_results": [],
    "omitted_role_results": 0,
    "outputs_evicted": 0,
    "confirmed_checkpoint": {
      "accepted_stage": { "number": 10, "name": "seed" },
      "pending_transition": { "direction": "up", "stage": { "number": 200, "name": "finish" } },
      "workflow_started": true
    },
    "attempted_checkpoint": null,
    "failure": {
      "kind": "process_failed",
      "message": "verify-up exited with status 9",
      "stage": { "number": 200, "name": "finish" },
      "role": "verify-up"
    },
    "verification_choices": {
      "retry": { "direction": "up", "target_stage": 200 },
      "reverse": { "direction": "down", "target_stage": 10 }
    }
  }
}
```

`state` is `running`, `complete`, `stopped`, or `unavailable`; it is separate
from HTTP delivery status. `active_role` is null or a stage identity plus role.
Each `role_results` item contains its stage and role, process state
(`in_progress`, `succeeded`, `failed`, or `launch_failed`), optional exit code,
message and elapsed milliseconds, output ID/state, original stdout/stderr byte
counts and per-stream truncation flags. Output state is `not_returned`,
`available`, `unavailable`, or `evicted`. `confirmed_checkpoint` records the last
checkpoint reported by Workbench; `attempted_checkpoint` is populated when a
save failed. Neither a successful role event nor a successful HTTP response
alone implies accepted movement.

Movement errors use `{ "error": { "code": string, "message": string },
"operation_id": string | null }`. Other API errors use
`{ "error": { "code": string, "message": string } }`. Common movement codes are
`malformed_request`, `invalid_direction`, `invalid_target`, `unknown_workspace`,
`workspace_unavailable`, `workspace_busy`, and `movement_task_failed`.

The workspace GET returns the project/workspace identity, checkpoint, selected
stage number, ordered stages and definitions, plus `server_instance_id`,
`observation_revision`, `movement_busy`, and the latest process-local observation.
`storage_issue` is null after a successful read. If a previously readable
workspace becomes unavailable, the host may return its last readable view with
`storage_issue` set; the view is labeled as cached and must not be treated as a
new database read. A first read failure remains a typed API error.

The SSE stream first sends `snapshot` with workspace/server IDs, revision, busy
state, the checkpoint paired with that revision, and the latest observation.
Clients reconcile that checkpoint with the observation before presenting a
movement result. On a save failure, the observation's `confirmed_checkpoint` is
the last position Workbench confirmed and `attempted_checkpoint` shows the
unconfirmed values; the host retains both even if a later storage read fails.
Subsequent `movement.started`, `role.started`,
`role.finished`, and `movement.finished` events are compact invalidations; `resync`
contains a fresh snapshot after subscriber lag. Role-finished events identify the
completed role and its output reference but never carry child bytes. The browser
uses the GET/output routes to read current state and retained bytes.

Movement request direction must be `up` or `down`; unknown targets and malformed
requests return 400. Unknown workspaces return 404, unavailable workspaces 503,
and overlapping UI movements for one workspace 409 `workspace_busy`. A completed
movement that stopped inside Workbench returns a structured 2xx observation with
`state: stopped`; HTTP delivery success is not movement success. Core execution
revalidates the direction and numeric target against current checkpoint state.
Reads do not mutate a workspace, apply migrations, bootstrap a database, or run
authored code. Output routes return raw `application/octet-stream`, separate
stdout from stderr, set an original-byte-count header, and explicitly mark
truncation. An evicted or expired output returns 410.

SSE connects before sending its first snapshot, closing the subscribe/snapshot
race. Events include workspace ID, server incarnation, monotonic workspace
revision, operation ID, direction/target, and stage/role where applicable. Event
payloads are small invalidations; the UI resynchronizes from the read API rather
than treating SSE as durable replay. A lagged stream receives a fresh snapshot.
Closing or reconnecting the stream never submits or repeats a movement.

Each launch binds 127.0.0.1 on an ephemeral port and prints the usable URL before
attempting to open it with macOS open. The startup credential is carried only in
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
tab or reconnect receives the current busy state and latest observation.

Press Ctrl-C in the launching terminal to close the listener when idle. Shutdown
does not request rollback or promise a new child-cancellation guarantee. Forced
termination and interruption of an active role do not prove what external effects
occurred; inspect the workspace and durable checkpoint yourself. Concurrent direct
CLI mutation of a UI-active workspace is unsupported. There is no interprocess
lock, multi-instance coordination, live byte stream, durable attempt history,
project switching, or preference migration across ephemeral-port origins.

## Design references

The [UI reference review](../research/2026-10-02-ui-reference-review.md) retains
publisher-attributed screenshot links and patterns borrowed from Dagu, Inngest,
Decagon and Playwright. Remote images are source links, not runtime assets or
screenshots newly captured by this implementation.
