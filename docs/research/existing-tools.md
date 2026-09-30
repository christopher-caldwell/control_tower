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
method: Official documentation review; no local product trials performed in this documentation
  pass
---

# Existing tools and stateful execution patterns

## Scope and evidence limits

This revisits all substantive comparators discussed: Dagu, Runme, Kestra, GitHub Actions, Kreya, Bruno, and Windmill. Swagger, Postman, and Retool were category analogies, not validated state-protocol candidates. The findings below come from official documentation accessed on September 30, 2026; they are not hands-on benchmarks or a claim that no other tool fits.

Product facts are described alongside sources. The implications for Control Tower are our interpretation. Detailed installation, pricing, licensing, AI/MCP setup, and version-specific UI walkthroughs from the earlier chat are not carried forward as operational instructions. They should be checked against the chosen version when needed.

## Dagu

**Observed.** Dagu's [shell/direct-execution documentation](https://docs.dagu.sh/step-types/shell) supports ordinary commands, executable files, explicit-argument direct execution, and shebang-bearing script blocks. Built-in HTTP and SQL actions are optional, not a prerequisite for external scripts.

Its [outputs contract](https://docs.dagu.sh/writing-workflows/outputs) gives each attempt a `DAGU_OUTPUT_FILE`. Command outputs are declared and written by name, with multiline and JSON validation options. Failed or incomplete attempts do not publish step outputs. Selected-step execution can reuse successful outputs from a previous run or accept supplied values.

[Persistent state](https://docs.dagu.sh/writing-workflows/persistent-state) is separate: small JSON values survive across runs, have versions and scopes, and live in a file-backed store by default. Its `state.diff` operation may also update the stored value unless configured otherwise; it is not simply a universal historical-diff viewer.

[Approval push-back](https://docs.dagu.sh/writing-workflows/approval) resets selected execution records and downstream dependents and reruns them. This documented bookkeeping is not an authored inverse of each external mutation.

**Implication.** Borrow the separation among attempt output, persistent context, and files. Do not copy failure-output suppression without addressing recovery IDs. Test a script-only Dagu recipe before treating authoring weight as an established limitation. The narrower hypothesis is that verified-fixture navigation may fit the development loop better than managing workflow runs.

## Runme

**Observed.** [Runme's walkthrough](https://docs.runme.dev/resources/walkthrough/) supports independently runnable notebook cells, environment-aware sessions, and a Reset Session control. Its [piping documentation](https://docs.runme.dev/usage/pipes-variables/) exposes the preceding result as `$__` and qualifying named-cell outputs as environment variables. It does not share ordinary language-level variables across cells as an in-process notebook kernel would.

**Implication.** Named values and interactive continuity are relevant. Positional “last output” is less attractive when inspections and unrelated actions are interleaved. Resetting session variables must not be confused with repairing external data. A plain spawned executable will not reproduce Runme's session capture merely by using `export`.

## Kestra

**Observed.** [Execution context for scripts](https://kestra.io/docs/scripts/execution-context) can materialize execution metadata as a JSON file that languages can read without an SDK. It contains inputs and earlier outputs, among other execution metadata. The documented lifecycle also addresses sensitive values and removal of the file. [Replay](https://kestra.io/docs/concepts/replay) creates an execution starting at a selected task with reuse of earlier execution information.

**Implication.** Pass data as structured input rather than substituting it into source code. A context snapshot is not an external-state snapshot. Keep our metadata much smaller than a complete orchestration execution model. Reusing an ID after replay does not prove the referenced fixture remains valid.

## GitHub Actions

**Observed.** GitHub documents [environment-provided file channels](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands): `GITHUB_OUTPUT` for outputs, `GITHUB_ENV` for values available to subsequent steps in the job, and `GITHUB_STEP_SUMMARY` for presentation. `GITHUB_STATE` is specifically for an action's pre/main/post lifecycle; it is not a general durable workspace store.

**Implication.** A small file protocol can work without an SDK and without making stdout a control channel. Do not confuse GitHub's “state” with a persistent fixture session, and do not copy its many channels or workflow-expression syntax simply because its output-file mechanism is useful.

## Kreya

**Observed.** [User variables](https://kreya.app/docs/user-variables/) hold JSON-serializable values in a project-scoped store, separate from environment configuration. They are saved on exit and survive restart in local application data. The documentation marks this feature Pro/Enterprise. Variables are independent of the currently active environment.

**Implication.** Remembering an entity ID across manual operations is a strong comparator. However, project scope alone can be insufficient if the same recipe targets different environments. Do not equate save-on-exit with crash-safe persistence, or imply the cited feature is necessarily available in the free edition.

## Bruno

**Observed.** [Bruno's variables documentation](https://docs.usebruno.com/variables/overview) describes multiple scopes and precedence, with separate access rules for process environment and prompt variables. Storage differs by variable type. Current documentation also describes typed variables, so a blanket “all values are strings” characterization would be inaccurate.

**Implication.** Separate configuration from transient runtime values without importing a large precedence stack. The owner explicitly rejected Bruno as the desired product fit; it remains a useful comparison of scoping decisions, not the recommended endpoint for this workbench.

## Windmill

**Observed.** Windmill distinguishes [variables, secrets, and environment/contextual values](https://www.windmill.dev/docs/core_concepts/variables_and_secrets). Its [state resources](https://www.windmill.dev/docs/core_concepts/resources_and_types#states) retain JSON values across script executions with a context-derived identity. The documented convenience APIs use Windmill clients.

**Implication.** Persistence scope must be deliberate: which script, fixture, user, or execution does a value belong to? Borrow that question, not the platform resource model or SDK. A local file protocol remains a smaller candidate for this project's boundaries.

## Cross-tool findings

| Pattern | Why it matters here | What it does not establish |
| --- | --- | --- |
| Dedicated machine-output channel | Keeps ordinary command output usable. | The correct encoding, deletion rule, or failure policy. |
| Structured process input | Preserves types without templating source. | The storage backend or need for every context field. |
| Named persistent runtime values | Avoids recopying IDs between interactive actions. | Continued validity of those IDs or a verified stage. |
| Separate configuration and context | Reduces accidental coupling of endpoints and fixtures. | A need for many scopes or a secrets-management platform. |
| Attempt history | Helps explain how a value arose. | External rollback, exactly-once execution, or complete recovery. |

## Corrections to earlier comparisons

The earlier contrast between Dagu's built-in drivers and our scripts was too strong. Direct/script execution is a documented path. The authoring difference still needs a fair trial.

“Rewind” and “reset” were also sometimes used too loosely. Execution bookkeeping, variable clearing, external compensation, and fresh-fixture creation are distinct operations. Likewise, no reviewed documentation proves that every desired interaction is convenient in the product UI.

The available evidence supports specific reusable patterns, not a verified market gap or a claim that Control Tower must be built. See [Q6](../design/open-questions.md#q6--does-the-interaction-justify-a-custom-tool) for the fair comparison exercise.
