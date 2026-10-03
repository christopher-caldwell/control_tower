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

Run `control-tower ui` from the Project directory: the directory that contains
`workspaces/`. The host binds IPv4 loopback on an available port and prints a plain
URL. Open that URL manually in a browser. There is no browser launch command, URL
token, session exchange, or runtime frontend server.

From a checkout, the equivalent command is:

```sh
cd /path/to/project
/path/to/control_tower/target/debug/control-tower ui
```

The launch inventory is the directory names directly under `workspaces/`. The UI
lists directories without opening their databases, reading their stages, or running
scripts. A missing or empty inventory produces an empty state. The list is fixed at
startup; restart the UI to pick up added or removed workspaces.

Selecting a workspace reads that workspace's current state and stages. An
unprepared or malformed workspace reports its ordinary error when selected; it does
not block the UI or a healthy sibling. Listing, selection, and status do not bootstrap,
migrate, repair, or verify storage. Prepare each workspace explicitly before using
its movement controls:

```sh
control-tower-db bootstrap-local workspaces/my-workspace
control-tower-db migrate-local workspaces/my-workspace
control-tower-db verify-local workspaces/my-workspace
```

Each workspace uses the existing `stages/<number>-<name>/` layout and its own
SQLite checkpoint. Stage numbers are ordered identities; gaps such as 10 and 200
are allowed. The UI does not traverse example dependencies or require their
runtimes to list workspace names.

## Workspace shell and movement controls

The desktop shell shows the launch Project, a manually collapsible workspace rail,
an ordered stage narrative, a persistent selected-stage inspector, and a progression
action region. It has no mobile or tablet layout. The current inspector includes a
local width control; rail collapse and inspector width are browser-local display
preferences.

Selecting a workspace opens its live view. Selecting any stage changes inspection
only: it does not execute a script, write a checkpoint, or change the movement target.
The accepted checkpoint remains separate from the selected stage. Pending transitions
show their direction with the last accepted position, while future stages remain
inspectable without running them.

The action region consumes Application's immediate `movement_choices`. It names the
next stage or the stage being backed out and keeps sparse stage numbers intact. React
does not reconstruct legal movements from selected stage, checkpoint arithmetic, or
definition files. A verifier failure observed in the current attempt can supply
Application-owned retry-only verification and reversal choices. Mutation and
verification results are shown separately. A cold pending checkpoint does not
fabricate a verifier failure, and a historical accepted stage without retained role
evidence is shown as applied with history unavailable.

The selected-stage inspector shows checkpoint state, mutation and verification
results, relevant stdout/stderr, and the stage's executable definitions. Reading a
definition is inspection only. Full normal file contents are displayed as escaped
text; ordinary read errors are shown beside the affected role. No fixed-size preview
or truncation contract is applied.

## HTTP and observations

The API uses the same loopback origin as the embedded UI and has four capabilities:

| Method and path | Purpose |
| --- | --- |
| GET `/api/project` | Return the startup Project name and workspace directory identities. |
| GET `/api/workspaces/{id}/events` | Send the selected workspace's current snapshot and later changes as SSE `snapshot` events. |
| POST `/api/workspaces/{id}/movements` | Submit `{direction, target_stage, expected_checkpoint}`. |
| GET `/api/workspaces/{id}/stages/{number}` | Read the selected stage's full definitions through Workbench. |

The movement request copies `completed_stage_count`, `uuid`, and `pending` from the
shown checkpoint. Application compares that complete expected state to the fresh
stored state before it launches a role or writes a checkpoint. A mismatch returns
409 `stale_checkpoint`; an overlapping request for the same workspace returns 409
`workspace_busy`. Both use `{ "error": { "code": string, "message": string } }`.
A completed request returns 204. HTTP does not decide legal movement targets or call
persistence directly.

SSE begins with the current selected-workspace snapshot; reconnecting observes state
without replaying commands. The snapshot carries status, movement choices, busy
state, the latest movement outcome, role results, and buffered role output. Starting
and finished callbacks report semantic role state. Stdout and stderr are attached as
separate displayable text when that role returns. Normal text is decoded lossily at
the outer UI boundary and rendered as inert text. There is no raw-byte download API,
output identifier/cache, WebSocket, or byte-by-byte process stream.

Each workspace has one in-process admission guard. It covers the blocking Workbench
operation and final snapshot publication, including when the browser request ends.
This is local single-instance coordination; there is no interprocess lock, queue,
durable execution history, or cancellation framework. A stopped or failed movement
is reported with its confirmed checkpoint and ordinary error details. Process results
do not imply checkpoint acceptance when Application could not complete or save the
movement.

Project inventory is separate from per-workspace status. The browser loads names
once, then observes only the selected workspace. A new SSE subscription reads a
fresh selected-workspace snapshot; it does not fan out status reads across siblings.
The Rust host constructs Workbench inside each blocking status, movement, or
inspection operation, while Entry selects Database and Infrastructure
implementations.

Stop the UI host from its launching terminal when finished. Stopping the host does
not roll back author-owned effects; inspect the workspace and external systems if a
role was interrupted. Closing a tab or losing a POST response does not itself cancel
a movement, and the browser does not retry a movement automatically.

## Design references

The [UI reference review](../research/2026-10-02-ui-reference-review.md) retains
publisher-attributed source screenshots and patterns borrowed from Dagu, Inngest,
Decagon, and Playwright. The links are design context rather than runtime assets or
newly captured implementation screenshots.

The [macOS delivery validation](../research/2026-10-03-macos-ui-delivery.md) is a
record of its original platform-specific run. Its platform and screenshot checks are
historical evidence, not completion requirements for the current loopback host.
