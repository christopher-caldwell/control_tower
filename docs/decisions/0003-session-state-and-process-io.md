---
id: ADR-0003
title: Separate persisted context from process input and output
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-unapproved-details
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../research/existing-tools.md
---

# ADR-0003: Separate persisted context from process input and output

## Standing and proposal

The user endorsed this general approach as the leading candidate for further research, not as a finished wire protocol. Persist a small workbench-owned session context; expose a snapshot to each executable; let the executable propose changes through a separate output file; capture stdout/stderr for ordinary results. Environment variables can carry paths and possibly convenient scalar inputs.

`WB_CONTEXT`, `WB_OUTPUT`, and all payload examples below are **illustrative working names**. Output encoding, merge behavior, failure publication, persistence backend, and automatic environment projection remain undecided.

## Distinct kinds of information

| Kind | Meaning | Candidate treatment |
| --- | --- | --- |
| Configuration | Inputs such as the selected API endpoint or database connection. | Author-supplied; not silently mixed into saved fixture context. |
| Invocation inputs | Values chosen for this particular action. | A separate input view; overrides need an explicit rule. |
| Session context | IDs and other small values intentionally carried between actions. | Persisted by the workbench if restart persistence is adopted. |
| Controller metadata | Navigation and verification evidence. | Controlled by the workbench, not arbitrary script output. |
| Attempt result | Logs, exit result, timestamps, and proposed outputs from one invocation. | Retained evidence, distinct from active context. |
| Artifacts | Reports, dumps, or other substantial files. | Keep as files; a managed artifact directory is deferred. |

These distinctions do not require six stores or a variable-precedence hierarchy. A single small persistence mechanism could hold several categories. The distinction is who owns each value and when it is trusted.

## Input alternatives

| Mechanism | Strength | Cost for this tool |
| --- | --- | --- |
| Environment variables | Convenient for small script inputs and locating files. | Values need string encoding; names, limits, inheritance, and accidental exposure need rules. |
| Command-line arguments | Natural interface for existing executables. | Binding every action's private argument syntax can bloat configuration. |
| Structured stdin | One language-neutral input message. | Occupies the script's input stream and can interfere with tools that already consume it. |
| Context file | Typed, inspectable data without consuming stdout or stdin. | Requires parsing and a defined lifecycle; shell scripts may need a utility. |
| Live shared mutable file | Minimal initial mediation. | Unclear write ownership, partial updates, and poor attribution of changes. |

Prefer a context file as the canonical process input view, with environment-provided paths. Normal process environment inheritance is not a return channel: a child receives its own environment rather than a shared store. See the [environment model](https://man7.org/linux/man-pages/man7/environ.7.html). Runme's captured session environment is additional runner behavior, not something a plain subprocess provides automatically.

Do not eagerly flatten every context field into environment variables. The convenient `WB_USER_ID` example raises real questions about reserved names, `user_id` versus `USER_ID`, nested values, nulls, and secrets. Start with the authoritative structured view; keep projections optional until the authoring experiment earns them.

## Output alternatives

| Mechanism | Strength | Cost / unresolved rule |
| --- | --- | --- |
| Structured stdout | Familiar Unix composition; no extra output file. | Incidental prints or child-tool output can break parsing. Still valid for scripts deliberately designed around it. |
| `KEY=value` output file | Very easy to produce in shell. | Types, multiline values, deletion, and duplicate keys require extra conventions. |
| One JSON output document | Familiar serializers; supports nested values and clear validation. | Whole-state replacement can lose keys; patches need defined semantics; interrupted writes can be incomplete. |
| JSON Lines | Can retain incrementally emitted records. | Introduces ordering, partial-record, and per-record commit questions. |
| SDK or local RPC | Rich operations and live interaction. | Adds coupling and infrastructure without a demonstrated need. |

The leading choice is the **separate output channel**, not yet a particular encoding. Dagu and GitHub Actions establish precedent for runner-provided output paths; their exact publication rules are not automatically correct for an interactive fixture workbench. See [the comparison](../research/existing-tools.md).

For example, an explicit change document could distinguish deletion from null:

```json
{
  "set": {"user_id": "example-user", "note": null},
  "unset": ["record_id"]
}
```

This illustrates a design question, not selected syntax. A full replacement document would be simpler in some cases but needs safeguards against accidentally dropping unrelated values. Implicit deep merges also need rules for arrays and deleted nested fields.

## Candidate lifecycle and the important failure gap

For each invocation, the workbench prepares a context snapshot and a fresh output destination. The executable can print normal results and write proposed state changes. The workbench records the attempt, parses the output as data, and decides whether to adopt it. Scripts do not directly set the controller's stage number.

For a transition, a destination verifier may need the proposed context, including newly created IDs, before the stage can be accepted. A possible policy is to validate the proposal, run verification against that candidate context, then accept context and checkpoint together. On failure, retain the proposal as recovery evidence and mark the fixture uncertain. This is a candidate sequence, not an approved all-or-nothing guarantee.

Consider a creation script that emits a new ID and then fails. Blindly discarding its output can lose the handle needed to clean up the created record. Blindly promoting its output can misrepresent success. Therefore distinguish **recorded evidence**, **accepted working context**, and **verified fixture conditions**. Exact promotion and manual recovery behavior are the joint [Q1/Q2 decisions](../design/open-questions.md#q1--what-exactly-does-an-action-publish).

A crash after an external mutation but before any output is written cannot be repaired by local metadata alone. Authors may need an external lookup or an idempotency strategy. No rollback of the workbench's JSON can undo that API call.

## Ownership, safety, and persistence

The input snapshot is read-only by contract; it is not a security boundary against a script running as the same user. The output channel is data, never shell code to `source` or evaluate. stdout may itself contain JSON or tables; reserving it for visible results does not require prose-only logs.

Local context persistence should preserve useful IDs across restarts, but restart persistence is still a candidate. A persisted verification is historical and may need rechecking. Keep runtime files separate from tracked recipe files, avoid saving the full inherited environment, and keep credentials out of fixture state by design rather than assuming that local files are secret storage.

JSON on the process boundary does not imply JSON as the storage backend. SQLite, ordinary files, atomic write strategy, retention, and workspace/environment binding remain open. So does process concurrency: a simple serial mutation policy is a candidate, not a distributed-lock requirement. A second application instance must not silently bypass whatever ownership rule is chosen.

## Confirmation

Exercise shell and Node authoring with scalar, structured, multiline, null, and deleted values. Then test invalid output, nonzero exit after publishing an ID, assertion failure, interrupted execution, restart, and a changed recipe. Prefer the smallest protocol that reports these cases honestly. Do not add live RPC, automatic retries, or an event-sourcing model merely because the failure cases exist.
