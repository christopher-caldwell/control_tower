---
id: CT-REF-BROWSER-WORKBENCH
title: Browser workbench
type: reference
status: maintained
created: '2026-10-02'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../crates/cli/src/web.rs
- ../../crates/cli/src/web/http.rs
- ../../ui/src/workbench.tsx
- ../decisions/0006-loopback-web-ui.md
- ../decisions/0007-desktop-ui-shell.md
- ../research/2026-10-02-ui-reference-review.md
---

# Browser workbench

## Build the packaged UI

The UI is a desktop browser client served by the ordinary Rust `control-tower`
executable. React assets are built ahead of time and embedded into that executable;
the running host does not require Node, Vite, SSR, or a separate frontend server.

From a Control Tower checkout, rebuild the React assets and Rust executable with:

```sh
cd ui
npm ci
npm run build
npm test
npm run test:browser
cd ..
cargo build --locked --workspace
```

The committed `ui/dist/` is embedded by Rust builds. Rebuild it after changing UI
source. Browser tests use the existing Playwright setup; they are test-time tooling
only.

## Start the UI

Run `control-tower ui` from the Workspace directory: the directory that contains
`workflows/`. The host binds IPv4 loopback on an available port and prints a plain
URL. Open that URL manually in a browser. There is no browser launch command, URL
token, session exchange, or runtime frontend server.

From a checkout, the equivalent command is:

```sh
cd /path/to/workspace
/path/to/control_tower/target/debug/control-tower ui
```

The launch inventory is the directory names directly under `workflows/`. The UI
lists directories without opening their databases, reading their stages, or running
scripts. A missing or empty inventory produces an empty state. The list is fixed at
startup; restart the UI to pick up added or removed workflows.

Selecting a workflow reads that workflow's current state and stages. An
unprepared or malformed workflow reports its ordinary error when selected; it does
not block the UI or a healthy sibling. Listing, selection, and status do not bootstrap,
migrate, repair, or verify storage. Prepare each workflow explicitly before using
its movement controls:

```sh
control-tower-db bootstrap-local workflows/my-workflow
control-tower-db migrate-local workflows/my-workflow
control-tower-db verify-local workflows/my-workflow
```

Each workflow uses the existing `stages/<number>-<name>/` layout and its own
SQLite checkpoint. Stage numbers are ordered identities; gaps such as 10 and 200
are allowed. The UI does not traverse example dependencies or require their
runtimes to list workflow names.

## Workflow shell and movement controls

The desktop shell shows the launch Workspace, a manually collapsible workflow rail,
an ordered stage narrative, a persistent selected-stage inspector, and a progression
action region. It has no mobile or tablet layout. The inspector has a fixed CSS
width. Both rails can be collapsed and reopened while the UI is running;
collapse state resets when the page reloads.

Selecting a workflow opens its live view. Selecting any stage changes inspection
only: it does not execute a script or write a checkpoint. Selection is primarily
inspection state; a selected future stage may also name the target of an explicit
Run to Stage X action. The accepted checkpoint remains separate from the selected stage. Pending transitions
show their direction with the last accepted position, while future stages remain
inspectable without running them.

The action region consumes Application's immediate `movement_choices`. It names the
next stage or the stage being backed out and keeps sparse stage numbers intact. When
no verification is pending, **Run next** submits the immediate upward choice. **Run to
Stage X** appears when the selected stage is a future stage farther than the next one,
and **Run all** appears when the final stage is future and farther than the next one.
Each submits one ordinary upward movement request, using the same expected checkpoint,
whose `target_stage` is the selected or final stage. The existing Application movement
engine validates the target, walks the intermediate stages, verifies each, updates the
checkpoint, and stops on the first failure; React does not sequence stages itself or
reconstruct legal movements from checkpoint arithmetic or definition files. Pending-verification recovery (retry-only verification and backout) is unchanged and
remains separate from these controls. A verifier failure observed in the current attempt can supply
Application-owned retry-only verification and reversal choices. Mutation and
verification results are shown separately. A cold pending checkpoint does not
fabricate a verifier failure, and a historical accepted stage without retained role
evidence is shown as applied with history unavailable.

The selected-stage inspector shows checkpoint state, mutation and verification
results, relevant stdout/stderr, and the stage's executable definitions. Reading a
definition is inspection only. Full normal file contents are displayed as escaped
text; ordinary read errors are shown beside the affected role. No fixed-size preview
or truncation contract is applied.

If current status cannot be read, the stage list, checkpoint and movement choices
are omitted and movement stays disabled. The status error is shown, and the latest
role results this host observed remain in the inspector with their separate
stdout/stderr, labelled as observed output rather than current state.

## HTTP and observations

The API uses the same loopback origin as the embedded UI and has four capabilities:

| Method and path | Purpose |
| --- | --- |
| GET `/api/workspace` | Return the startup Workspace name and workflow directory identities. |
| GET `/api/workflows/{id}/events` | Send the selected workflow's current snapshot and later changes as SSE `snapshot` events. |
| POST `/api/workflows/{id}/movements` | Submit `{direction, target_stage, expected_checkpoint}`. |
| GET `/api/workflows/{id}/stages/{number}` | Read the selected stage's full definitions through Workbench. |

The movement request copies `completed_stage_count`, `uuid`, and `pending` from the
shown checkpoint. Application compares that complete expected state to the fresh
stored state before it launches a role or writes a checkpoint. A mismatch returns
409 `stale_checkpoint`; an overlapping request for the same workflow returns 409
`workflow_busy`. Both use `{ "error": { "code": string, "message": string } }`.
A completed request returns 204. HTTP does not decide legal movement targets or call
persistence directly.

SSE begins with the current selected-workflow snapshot; reconnecting observes state
without replaying commands. The snapshot carries status, movement choices, busy
state, the latest movement outcome, role results, and buffered role output. Starting
and finished callbacks report semantic role state. Stdout and stderr are attached as
separate displayable text when that role returns. Normal text is decoded lossily at
the outer UI boundary and rendered as inert text. There is no raw-byte download API,
output identifier/cache, WebSocket, or byte-by-byte process stream.

Each workflow has one in-process admission guard. It covers the blocking Workbench
operation and final snapshot publication, including when the browser request ends.
This is local single-instance coordination; there is no interprocess lock, queue,
durable execution history, or cancellation framework. A stopped or failed movement
is reported with the current checkpoint and ordinary error text. Process results do
not imply checkpoint acceptance when Application could not complete or save the
movement.

Workflow inventory is separate from per-workflow status. The browser loads names
once, then observes only the selected workflow. A new SSE subscription reads a
fresh selected-workflow snapshot; it does not fan out status reads across siblings.
The Rust host constructs Workbench inside each blocking status, movement, or
inspection operation, while Entry selects Database and Infrastructure
implementations.

Stop the UI host from its launching terminal when finished. Stopping the host does
not roll back author-owned effects; inspect the workflow and external systems if a
role was interrupted. Closing a tab or losing a POST response does not itself cancel
a movement, and the browser does not retry a movement automatically.

## Design references

The [UI reference review](../research/2026-10-02-ui-reference-review.md) retains
publisher-attributed source screenshots and patterns borrowed from Dagu, Inngest,
Decagon, and Playwright. The links are design context rather than runtime assets or
newly captured implementation screenshots.
