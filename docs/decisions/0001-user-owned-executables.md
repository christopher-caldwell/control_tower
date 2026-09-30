---
id: ADR-0001
title: Keep application operations in user-owned executables
type: decision
status: accepted
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: explicit-user-direction
decision_date: '2026-09-30'
sources:
- ../history/2026-09-30-initial-design.md#e07-the-authoring-boundary
- ../history/2026-09-30-initial-design.md#e08-parallel-tracks-and-shebang-direction
---

# ADR-0001: Keep application operations in user-owned executables

## Decision and scope

Lead with executable files whose shebang selects their runtime. The author owns the scripts and application-specific behavior. Control Tower provides execution controls, navigation, visible results, and a small shared-context boundary rather than its own HTTP/SQL driver language.

This records the user's authoring direction at 03:08:00Z and explicit shebang-first choice at 04:05:56Z on September 30, 2026. It does not accept a detailed process protocol, promise cross-platform support, or freeze future extension points.

## Context

The original pain is coordinating existing operations during development. Reexpressing SQL and API requests inside a platform-specific schema can move that work rather than remove it. The developer already has code, tools, and runtimes that understand the application.

The workbench must not edit application source. The author may use an external coding assistant to prepare ordinary scripts, but the workbench does not need an AI service to execute them.

## Alternatives and tradeoffs

| Alternative | Advantage | Reason not selected as the core |
| --- | --- | --- |
| Built-in HTTP and database actions | Centralized credentials, validation, and result formatting. | Requires integrations and an authoring API unrelated to the core interaction problem. |
| Embedded language or mandatory SDK | Richer typed interaction and convenient helpers. | Couples every action to the workbench and complicates reuse. |
| Inline command strings in configuration | Convenient for tiny recipes. | Moves quoting and script authoring into configuration; not the user's preferred file-led direction. |
| User-owned executable files | Ordinary editing, reuse, language choice, and project tooling. | Authors manage dependencies, meaningful errors, and result publication. Selected despite those costs. |

Dagu is not disqualified by these alternatives: its command execution path also supports ordinary scripts. The comparison must concern the total authoring and interaction experience, as documented in [Existing tools](../research/existing-tools.md#dagu).

## Consequences

The same action can use shell utilities, Node, Python, a project CLI, or application libraries without Control Tower understanding those tools. It should not require source rewriting, generated API clients, or replacing working scripts with a special database interface.

Conversely, the runner cannot infer whether an HTTP call represented business success, whether a SQL change was safe, or whether a script is reversible. The author supplies those meanings. A successful process start is not successful action completion.

The shebang direction also does not eliminate executable permissions, interpreter availability, working-directory rules, environment setup, signal handling, or output capture. Nor is this a sandbox: scripts retain the permissions and access of their execution environment.

## Proposed implementation interpretation

Prefer launching a resolved executable path directly rather than constructing a shell command by substituting state into source text. Pass any arguments and environment values as data. Rust's [process API](https://doc.rust-lang.org/std/process/struct.Command.html) supports this separation and warns that relative program paths combined with a changed working directory can be ambiguous.

This is a proposed technical interpretation, not a new accepted process specification. Exact path resolution, missing-shebang behavior, compiled binaries, stdin, cancellation, and supported platforms still need a small contract. Runtime/dependency installation is not selected as a Control Tower responsibility.

## Confirmation and revisit trigger

Test a shell action and a Node action against the same recipe, run them manually as well as through the proposed boundary, and inspect a failed invocation. The author should not need a workbench-specific business API.

Revisit only if repeated real authoring pain justifies a narrow convenience. The existence of integrations in other tools is not sufficient evidence to add them here.
