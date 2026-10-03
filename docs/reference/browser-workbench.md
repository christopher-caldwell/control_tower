---
id: CT-REF-BROWSER-WORKBENCH
title: Browser workbench
type: reference
status: maintained
created: '2026-10-02'
updated: '2026-10-02'
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

## Current read-only interface

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

The inspector orders checkpoint state, mutation definitions, verification
definitions, captured output, and technical definition details. Definitions are
plain escaped text, capped at 128 KiB per role preview, and never executed while
being viewed. A configured role shows its definition path and Unknown result with
“No retained result”; this read-only phase has no execution evidence, including
for previously accepted stages and cold-start pending transitions. File presence
does not represent execution success. Missing optional verification is Not
configured.
This phase is read-only: output is unavailable and the primary progression control
is disabled. Execution, recovery and live observation arrive in a later phase.

## HTTP contract and session

The API serves JSON from the same IPv4 loopback origin as the bundled UI:

| Method and path | Purpose |
| --- | --- |
| POST /api/session | Exchange the startup token for a browser session cookie. |
| GET /api/project | List startup-discovered workspaces and their independent status. |
| GET /api/workspaces/{id} | Read checkpoint identity and ordered stage summaries. |
| GET /api/workspaces/{id}/stages/{number} | Read one stage role definitions. |

Errors use a JSON envelope with error.code and a human-readable error.message.
Invalid stage-number text returns 400; unknown workspaces or stages return 404;
unavailable workspaces return 503. Checkpoint and definition reads do not mutate
a workspace, apply migrations, bootstrap a database, or run authored code.

Each launch binds 127.0.0.1 on an ephemeral port and prints the usable URL before
attempting to open it with macOS open. The startup credential is carried only in
the URL fragment, which browsers do not send in HTTP requests. The frontend removes
the fragment and exchanges the credential for a random, HttpOnly, SameSite=Strict,
in-memory session cookie. There is no public token/configuration endpoint. Private
API reads require that cookie. API requests with an Origin header must match the
advertised origin; POST requests require it. Host must exactly match the loopback
host and ephemeral port. CORS is not enabled.

This protects against unrelated browser origins and unauthenticated local callers
accessing checkpoint details or definitions. It does not sandbox trusted stage
scripts or protect against malware already running as the same macOS user. Inspect
example scripts before use.

## Process and observation limits

The terminal owns the UI host process. Closing a browser tab or refreshing the page
does not stop the host. Press Ctrl-C in the launching terminal to close the idle
listener. This read-only phase launches no child roles, so it has no active stage
process to interrupt. Later execution support must keep operation lifetime separate
from browser lifetime and document active-shutdown limits; termination is not rollback.

This phase has no SSE stream, live child-byte streaming, durable attempt history,
or retained output. Refresh reads the current checkpoint again; it does not replay
a movement. The checkpoint database remains the source of accepted state. Cross-
restart preferences across ephemeral-port origins and project switching remain deferred.

## Design references

The [UI reference review](../research/2026-10-02-ui-reference-review.md) retains
publisher-attributed screenshot links and patterns borrowed from Dagu, Inngest,
Decagon and Playwright. Remote images are source links, not runtime assets or
screenshots newly captured by this implementation.
