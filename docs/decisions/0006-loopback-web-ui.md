---
id: ADR-0006
title: Use a loopback web UI as the first graphical driving adapter
type: decision
status: accepted
created: '2026-10-02'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
decision_authority: explicit-user-direction-after-targeted-source-review
decision_date: '2026-10-02'
sources:
- 0005-cli-first-driving-adapter.md
- https://github.com/dagucloud/dagu/blob/3cce8be87f2ad9b205105ecdef90a527fc70cb63/ARCHITECTURE.md
- https://github.com/dagucloud/dagu/blob/3cce8be87f2ad9b205105ecdef90a527fc70cb63/ui/src/hooks/useDAGRunSSE.ts
- https://github.com/inngest/inngest/blob/56f520014f6ed82d1f0630e213a8a2d35eeca64a/ui/packages/components/src/RunDetailsV4/RunDetailsV4.tsx
- https://v2.tauri.app/distribute/macos-application-bundle/
---

# ADR-0006: Use a loopback web UI as the first graphical driving adapter

## Context

Core v0 proved the workbench behavior through the CLI. A graphical interface is now justified as another Entry over the same Application behavior.

The relevant host choice is not “one Rust process versus two.” Both a Tauri UI and a local browser UI can package a built React frontend with one Rust application process. Vite and Node are development/build tools only; neither is required at runtime.

Dagu provides useful evidence for this shape: its frontend service exposes HTTP/SSE and embedded UI assets, and its ordinary DAG-run live view uses SSE. Inngest provides a useful counterexample: its ordinary run details still use HTTP polling even though the platform separately supports realtime transports. A WebSocket is therefore not a prerequisite for a responsive execution UI.

Control Tower is also unusually sensitive to process environment. Its core job is launching user-owned executables. A shell-launched local server naturally inherits the environment that launched it, while a macOS GUI application launched from Finder, Dock or Spotlight does not automatically receive the developer's shell PATH. Tauri can compensate for that, but doing so is additional application behavior rather than a free packaging benefit.

## Decision

The first graphical Control Tower UI will be a **local browser-based web UI served by Rust over loopback HTTP**.

The target runtime shape is:

```text
Control Tower executable
  -> UI Entry composition
     -> Application / Workbench
     -> loopback HTTP API
     -> SSE observations
     -> embedded built React assets
          -> manually opened browser
```

The production frontend is built ahead of time and embedded in, or packaged with, the Rust distribution. There is no Vite or Node runtime dependency.

The UI is another concrete driving adapter. It must invoke the same Application use cases as the CLI. It must not shell out to the CLI, recreate navigation rules, own verification semantics, or move business/process rules into HTTP handlers or React state.

The UI is launched through the ordinary Control Tower executable rather than a native app bundle. The host uses a generic loopback listener and prints a plain URL for manual opening; it does not launch the system browser or require a platform-specific app package.

## HTTP and live updates

Use ordinary HTTP request/response for commands and queries.

Use **Server-Sent Events (SSE)** for server-to-browser execution observations where immediate UI updates are useful. The current Application before/after role observations are the semantic source; the UI transport must not create a second navigation/execution model.

Do not add WebSockets unless a real bidirectional interaction earns them.

The first UI should surface role lifecycle promptly:

```text
role starts
-> UI shows running

role returns
-> UI shows success/failure
-> captured stdout/stderr becomes available
```

Stdout/stderr remain buffered per role as they are today. Byte-by-byte process-output streaming is **not** part of this decision. Add it only if real use shows that “running, then done — view output” is insufficient.

## Loopback boundary

The UI is a single-person local workbench, bound only to loopback. The host:

- bind only to loopback rather than a LAN-facing address,
- prefer an ephemeral port rather than a fixed public convention,
- serve the frontend and API from the same origin,
- print a usable URL without credentials.

The implementation has no session exchange, bootstrap token, Host/Origin enforcement, or CORS layer. The reconciled product scope is an ordinary local browser adapter rather than a local authentication or request-hardening feature.

## Why Tauri is not selected

Tauri remains a legitimate alternative. Its strongest advantages are:

- no listening HTTP control surface,
- explicit IPC/capability boundaries,
- native application/window lifecycle,
- native dialogs, drag/drop and other macOS integration.

Those benefits do not currently outweigh the added product machinery.

For Control Tower today, Tauri would add a desktop-app distribution/lifecycle path and would require deliberate handling of the shell environment used by user-owned executables. The local web design already provides a single Rust distribution, embedded React assets, no production frontend server process, and a straightforward HTTP/SSE delivery surface.

Tauri may be reconsidered if actual use establishes that native window ownership, deeper operating-system integration, or removal of the loopback HTTP surface is valuable enough to justify the additional environment and packaging behavior.

## High-level UI constraint

The host choice does not freeze the detailed visual design, but the first UI should preserve execution context rather than repeatedly navigating between full pages or collapsed panels.

The leading shape is:

```text
workflow/run context
+ ordered vertical stage rail
+ selected-stage inspector/output
```

Completed position and an active pending transition must remain visibly distinct. Stages are ordered migration-like states, not a DAG; the UI must not imply branching/dependency semantics that Control Tower does not have.

The desktop information architecture, workspace/workflow terminology, rail behavior and declarative progression controls are now settled by [ADR-0007](0007-desktop-ui-shell.md). Component library, exact styling and pixel sizing remain implementation/design tuning.

## Consequences

The next UI implementation can remain a thin Entry over the current architecture.

A browser tab closing does not define workbench state or external rollback. The Rust process owns execution and remains the relevant runtime lifecycle while the UI Entry is running.

No persistent run-history model, generalized event bus, terminal protocol, process-output streaming contract or native desktop packaging is introduced by this decision.
