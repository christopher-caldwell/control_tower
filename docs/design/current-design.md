---
id: CT-DESIGN
title: Current design and decision audit
type: design
status: maintained
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../history/2026-09-30-initial-design.md
- ../decisions/0001-user-owned-executables.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
---

# Current design and decision audit

## Product in one paragraph

Control Tower is a personal, local workbench for operating disposable development fixtures. A developer defines actions as executable files, then uses a UI to run an action, inspect its result, move through a sequence, and return to a useful test condition while changing application code elsewhere. Rust is the chosen implementation language; native versus browser-based UI is undecided. The tool does not implement the application's HTTP, SQL, or business operations, and it does not edit application source code.

The governing authoring idea is: **you run your own stuff; the tool supplies buttons, navigation, results, and working context.**

## The actual problem

A ticket requires a test user, associated records, and particular business states. A POST changes those records. The developer inspects the database, changes application code, and repeats. Eventually business rules make the fixture unsuitable and a new one is required.

Today the recipe is scattered among SQL snippets, terminal commands, copied IDs, and remembered ordering. The intended benefit is reducing that coordination cost, not building a replacement HTTP client, database client, or test assertion library.

A representative loop is:

```text
create fixture -> establish open record -> POST -> inspect
                                           |
                                change application code
                                           |
                    restore a useful fixture -> POST again
```

Running the whole sequence remains important. Individual execution is not merely a debugging fallback. These goals are recorded in [E01–E03](../history/2026-09-30-initial-design.md#e01-initial-problem).

## What is chosen, and what is still a candidate?

| Direction | Standing | Why it survives this review |
| --- | --- | --- |
| Personal use, local execution, Rust | User-chosen constraints | They bound the problem. Rust does not need a claim of superior performance to justify a personal choice. |
| Configured actions with clickable results | User-chosen goal | Directly removes the repeated copy/paste workflow. No configuration format has been selected. |
| User-owned executable files, led by shebang | User-chosen current direction | Preserves ordinary scripts and existing project tooling. See ADR-0001. |
| Try Dagu while exploring Control Tower | Explicit user decision | Learning from a tool and replacing it are different commitments. |
| Individual steps and full-sequence execution | Explicit user goal | Both are necessary for the stated development loop. |
| Migration-like up/down navigation and assertion gates | User-originated design direction | A small sequential model fits the examples. The complete semantics remain proposed in ADR-0002. |
| Persisted session context, read-only input snapshot, separate output file | Leading candidate endorsed for exploration | Named IDs can survive independent actions without mixing logs and updates. ADR-0003 is not a finalized protocol. |
| Automatic scalar environment projection, exact `WB_*` names, JSON shape | Unsettled | Convenience does not yet justify collision, typing, or exposure rules. |
| UI technology, storage backend, execution schema, dependencies | Unsettled | No prototype or measured constraint has earned these choices. |

## Minimal conceptual model

A **workspace** is the authored recipe and its executable references. A **session** is one working fixture context, distinct from both the recipe and an individual invocation. This distinction does not commit us to a multi-session management UI.

An **action** is an executable invocation. A **transition** uses an action to attempt a move between adjacent stages. A **stage** names a useful condition, optionally checked by author-supplied assertions. A **verification** checks the configured conditions without intentionally changing the fixture. An **attempt** is the evidence from one invocation: result, logs, and any proposed output values.

```text
                 AUTHOR-OWNED
    executable files, dependencies, API/SQL/business logic
                         |
                  process boundary
                         |
                CONTROL TOWER
    recipe + navigation + results + session context
```

These are conceptual responsibilities, not required Rust structs, crates, modules, or database tables.

## The leading state and invocation model

```text
persisted accepted context
        |
  per-invocation snapshot --------> WB_CONTEXT
                                     |
                                executable
                                 /       \
                        stdout/stderr    WB_OUTPUT
                              |             |
                         displayed       proposed changes
                                            |
                                  validate and adjudicate
                                            |
                                   accepted context
```

`WB_CONTEXT` and `WB_OUTPUT` are working names, not a compatibility promise. Scripts that only print a result need not use either. Configuration such as an API URL is conceptually different from runtime IDs; the context should not become a dump of the developer's entire environment.

The protocol is about passing data, not interpreting application behavior. HTTP response handling, SQL parameterization, imports from the application's own libraries, assertions, cleanup, and meaningful process exit codes remain the action author's responsibility.

## Every added mechanism has a cost

| Earlier idea | Challenge | Current disposition |
| --- | --- | --- |
| “The OS is the runner; this is almost nothing.” | Missing interpreters, execution permissions, working directories, cancellation, and output capture remain real responsibilities. | Keep the executable boundary, but acknowledge process lifecycle work. |
| “It is not a state machine.” | Ordered stages and transitions are a small state machine in the ordinary sense. | Avoid a generalized graph framework, not accurate terminology. |
| “Only the current stage number is needed.” | A failed mutation may leave neither source nor target state intact. | Distinguish last verification from present certainty; propose uncertain state handling. |
| “Down is the inverse of up.” | A compensating action may restore useful conditions without erasing history or external effects. | Author-defined compensation, not universal undo. |
| “Rebuild always creates a known state.” | Cleanup or setup can fail; clearing IDs does not clear the database. | Rebuild needs an authored recipe and target verification. No unconditional guarantee. |
| “Assertions define the state.” | Checks cover only what was authored, can overlap between stages, and can become stale. | Report configured checks passing at a time; do not claim exhaustive world-state knowledge. |
| “Utilities never affect the stage.” | An arbitrary utility can mutate the fixture or its identifying context. | Read-only inspection and out-of-band mutation cannot share an unconditional trust rule. |
| “The context file is read-only.” | A trusted local script still runs with the user's privileges. | Read-only is an ownership contract and accidental-write guard, not a sandbox. |
| “Env vars are universal storage.” | They are process inputs, not a typed, durable, bidirectional state store. | Use them as transport; decide optional projections separately. |
| “Every successful output becomes shared state.” | Large results, stale IDs, logs, and secrets do not all belong in current fixture context. | Explicit publication, distinct deletion semantics, and retained attempt evidence. |
| “State history is cheap and gives recovery.” | Ordering, retention, crashes, and external side effects complicate recovery. | A small attempt record is a candidate; event sourcing and time travel are not earned. |
| “Dagu owning drivers proves a gap.” | Its command and direct-exec paths do not require those drivers. | Test the actual interaction and authoring cost, not a caricature of Dagu. |
| “All configuration must be YAML.” | YAML appeared in examples, not a user selection. | File format and exact schema remain open. |
| “The LLM will write everything correctly.” | Generated scripts can be wrong or destructive. | External AI authoring is a convenience, not runtime authority or a correctness guarantee. |

Product observations are verified in [Existing tools](../research/existing-tools.md). The failure and protocol refinements above are assistant proposals derived from concrete counterexamples, not newly discovered user requirements.

## What has not earned scope

No built-in database/HTTP driver layer, SDK requirement, expression language, script-source templating, dependency installer, scheduler, distributed workers, generalized DAG engine, authentication platform, or built-in AI agent is selected. These are exclusions from the present design, not promises never to revisit a concrete need.

Likewise, no automatic retries, universal rollback, exact-once external execution, concurrent mutation model, rich artifact browser, or automated environment repair is implied by a simple UI. The process runs locally; the scripts' targets may still be remote. Local execution is not a guarantee that data or side effects remain on the machine.

## Evidence still required

The next task is not implementation of this entire document. It is the focused authoring/failure exercise in [Open questions and validation](open-questions.md), beginning with output publication and recovery semantics. A comparable Dagu experiment remains a parallel learning track, not a dependency or an abandoned alternative.
