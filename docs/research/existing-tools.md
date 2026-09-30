---
id: CT-RESEARCH-TOOLS
title: Existing tools and stateful execution patterns
type: research
status: recorded
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
verified_on: '2026-09-30'
method: Official documentation review plus user-reported local Dagu trial
sources:
- ../history/2026-09-30-initial-design.md#e11-dagu-trial
---

# Existing tools and stateful execution patterns

## Scope and evidence limits

The substantive comparators discussed were Dagu, Runme, Kestra, GitHub Actions, Kreya, Bruno, and Windmill. Swagger, Postman, and Retool were category analogies rather than validated state-protocol candidates.

External product facts below come from official documentation reviewed on September 30, 2026. The Dagu section additionally records the owner's direct local trial as a product-fit observation. That experience is not a benchmark or a claim that Dagu is badly designed.

## Dagu

**Documented behavior.** Dagu's [shell/direct-execution documentation](https://docs.dagu.sh/step-types/shell) supports ordinary commands, executable files, explicit-argument direct execution, and shebang-bearing script blocks. Built-in HTTP and SQL actions are optional.

Its [outputs contract](https://docs.dagu.sh/writing-workflows/outputs) provides a DAGU_OUTPUT_FILE channel for declared outputs. [Persistent state](https://docs.dagu.sh/writing-workflows/persistent-state) is separate and can retain small JSON values across runs. [Approval push-back](https://docs.dagu.sh/writing-workflows/approval) resets execution bookkeeping and reruns steps; it is not an authored inverse of each external mutation.

**Local trial outcome.** The owner installed/tried the prepared local Dagu setup and concluded that the product was close in capability but substantially heavier than the intended tool. Authentication/setup, server-style UI/runtime state, workflow management, and the broader orchestration model reinforced the distinction. The owner still considers Dagu useful and worth learning from.

**Implication.** Do not mischaracterize Dagu as requiring its database/HTTP drivers or as inherently cloud-hosted. The actual design lesson is narrower: Control Tower intentionally optimizes for walking a user-owned migration set, while Dagu solves a broader workflow/orchestration problem.

## Runme

**Observed.** [Runme's walkthrough](https://docs.runme.dev/resources/walkthrough/) supports independently runnable notebook cells, environment-aware sessions, and a Reset Session control. Its [piping documentation](https://docs.runme.dev/usage/pipes-variables/) exposes the preceding result and named-cell outputs through runner-managed mechanisms.

**Implication.** Interactive continuity and named values are relevant. Clearing session variables must not be confused with repairing external data.

## Kestra

**Observed.** [Execution context for scripts](https://kestra.io/docs/scripts/execution-context) can materialize execution metadata as a JSON file that scripts can read without a mandatory language SDK. [Replay](https://kestra.io/docs/concepts/replay) starts an execution at a selected task while reusing earlier execution information.

**Implication.** A structured input snapshot is a proven process-boundary pattern. Control Tower does not need Kestra's generalized execution history or replay model to borrow that idea.

## GitHub Actions

**Observed.** GitHub documents [environment-provided file channels](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands), including GITHUB_OUTPUT and GITHUB_ENV.

**Implication.** A runner-provided file can carry machine-readable updates without taking over stdout. Do not copy the many channels, expression language, or CI semantics unless a Control Tower use case requires them.

## Kreya

**Observed.** [User variables](https://kreya.app/docs/user-variables/) retain JSON-serializable values in a project-scoped store separate from environment configuration.

**Implication.** Remembering generated identifiers across manual operations is useful. Project scope and rich JSON types are not automatically the right Control Tower choices.

## Bruno

**Observed.** [Bruno's variables documentation](https://docs.usebruno.com/variables/overview) describes multiple scopes, precedence, storage rules, and typed values.

**Implication.** Configuration and runtime values are distinct, but a large precedence hierarchy is exactly the kind of product complexity this project should avoid until needed. The owner did not consider Bruno the intended product fit.

## Windmill

**Observed.** Windmill distinguishes [variables, secrets, and contextual values](https://www.windmill.dev/docs/core_concepts/variables_and_secrets) and has persistent [state resources](https://www.windmill.dev/docs/core_concepts/resources_and_types#states).

**Implication.** Persistence scope is an important question. A platform resource model and SDK are not necessary merely to retain a few fixture IDs.

## Database migration runners

**Flyway.** Current Flyway documentation describes a schema-history table that records migration versions, checksums and success/failure state, and separately supports optional undo migrations in reverse applied order. Its checksum validation is useful evidence that persisted migration bookkeeping must be tied to migration identity. Control Tower should borrow the structural-identity concern, but hard checksum enforcement on executable contents would conflict with its intended edit-and-retry development loop.

Sources: [schema history](https://documentation.red-gate.com/fd/flyway-schema-history-table-273973417.html), [undo migrations](https://documentation.red-gate.com/fd/undo-migrations-273973334.html).

**dbmate.** dbmate uses numerically ordered migrations with explicit up/down sections and a very small applied-version table. Its documentation notes that only the migration version is recorded and recommends rolling a migration back before changing its applied contents.

Source: [dbmate repository documentation](https://github.com/amacneil/dbmate).

**Implication.** The migration analogy holds up well for explicit author-owned up/down operations. Control Tower intentionally does **not** adopt production migration-history safeguards for v0: no durable applied-version table, checksum validation, or structural-drift protection. Its state is session-local and editing/rearranging files remains the author's responsibility.

## Cross-tool findings that survive

| Pattern | Useful lesson | Limit |
| --- | --- | --- |
| Dedicated machine-output channel | Keep human output and state updates separate. | Does not choose encoding or failure policy. |
| Structured process input | Pass context without templating source code. | Does not require a complex context object. |
| Named runtime values | Generated IDs can survive between manual actions in one Control Tower session. | Persistence across restarts is intentionally not required for v0. |
| Separate configuration and runtime context | Avoid mixing environment selection with fixture IDs. | Does not require many scopes. |
| Per-step results | stdout/stderr/exit history is useful during development. | Does not require a workflow execution engine. |

## Conclusion

The research no longer points toward “find the lightest workflow orchestrator.” The stronger product boundary is now explicit: Control Tower is a migration-style workbench that delegates operation semantics to user-owned executables.

Existing systems validate several small implementation patterns. They do not establish a need for orchestration machinery, and they do not eliminate the value of building a smaller tool for a personal workflow.
