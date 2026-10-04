---
id: ADR-0005
title: Use the CLI as the only v0 driving adapter
type: decision
status: accepted
created: '2026-09-30'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
decision_authority: explicit-user-direction-and-established-rust-playbook
decision_date: '2026-09-30'
sources:
- ../history/2026-09-30-initial-design.md#e21-cli-first-entry-adapter
- ../design/current-design.md#v0-entry-architecture-cli-only
---

# ADR-0005: Use the CLI as the only v0 driving adapter

## Decision

The CLI is Control Tower's first and only entry/driving adapter for v0.

It sits at the outer edge of the hexagonal architecture and invokes application use cases.

Future interfaces such as Tauri or an HTTP server are expected to be additional driving adapters over the same application/core behavior.

They are not part of the first implementation.

## Boundary

The CLI may own:

- argument/command parsing,
- terminal-oriented input,
- invoking the appropriate application use case,
- rendering stdout-style results, tables, status, and errors for the user.

The CLI must not own:

- up/down transition semantics,
- verify-up/verify-down rules,
- context checkpoint logic,
- active-transition state rules,
- storage decisions,
- process execution semantics beyond translating CLI intent into an application request.

Those belong inward.

## Composition root

The executable entry point can be the composition root.

For v0 it should explicitly construct:

- the SQLite session-state adapter,
- the process/executable adapter(s) required by the application,
- the application capability/service,
- the CLI adapter or command handlers that invoke the service.

The exact Rust module layout is not decided here.

No DI framework, container, plugin registry, or runtime adapter discovery is required.

## Future Tauri or HTTP entry layers

When another entry layer is added, the expected shape is:

~~~text
CLI ------\
           \
Tauri ------> application/core use cases
           /
HTTP ------/
~~~

Each adapter translates its transport/UI concerns into the same application-level commands/queries/use cases.

The application/core should not expose CLI-specific types merely because CLI arrived first.

Likewise, the CLI should not be written as a reusable “transport abstraction” prematurely. It is a concrete adapter.

## Why CLI first

CLI is the smallest surface for validating the actual workbench mechanics:

- discover a workflow,
- move up/down,
- run directional verification,
- inspect current session status/context/results,
- exercise failure cases.

It avoids committing to desktop/web UI technology before the interaction semantics are proven.

It also fits the project's local-only identity.

## What this does not decide

This ADR does not define:

- command names,
- subcommand hierarchy,
- output formatting,
- interactive TUI behavior,
- clap or another parsing library,
- shell completions,
- Tauri,
- an HTTP framework,
- public API stability.

Those details should follow the actual v0 use cases.

## Consequences

The first implementation can validate the full application boundary without UI framework complexity.

If application logic remains transport-independent, adding Tauri or HTTP later should mostly be an adapter/composition task rather than a rewrite.
