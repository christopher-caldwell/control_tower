---
id: CT-HISTORY-2026-09-30
title: Initial design discussion and corrections
type: history
status: recorded
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
correlation_id: CT-DISCUSSION-2026-09-30
source_window:
  start: '2026-09-30T00:49:35Z'
  end: '2026-09-30T05:03:39Z'
source_kind: Current conversation with relevant history recovery
---

# Initial design discussion and corrections

## Scope and provenance

This is a curated reconstruction of the discussion supplied to the documentation session, not a verbatim transcript. Times below are the user-message timestamps in UTC on September 30, 2026. The discussion occurred across the preceding local evening; UTC avoids relying on inconsistent local timezone labels in retrieved context.

`CT-DISCUSSION-2026-09-30` is a correlation label created for these docs. It is **not** a recovered ChatGPT conversation UUID. No verified chat URL was available. The source conversation can be identified by its opening request for a personal local Rust tool with clickable test actions and its later shebang and storage discussion. Relevant history recovery supplied no additional conflicting decision.

The repository was inspected on `main` at initial commit `29d1446806b7127c0e0bca471aa899c6021e2f46`; its only file was the short root README. The owner authorized this documentation work to be committed directly, entirely under `docs/`. Work-specific identifiers and actual data are intentionally replaced with generic fixture examples because the repository is public.

This record documents decision history, not a promised development schedule. No implementation milestones or deadlines were agreed.

## E01: Initial problem

**00:49:35Z — User direction.** A personal, local, Rust-based app, native or web, should expose configured test actions as buttons. The example was a test user with associated data that must be created, mutated, inspected, and eventually recreated because of business constraints. Current work was scattered between SQL snippets and terminal commands. Application source editing was outside the tool's role.

**Assistant interpretation.** Suggested a developer workbench rather than another HTTP client, and emphasized carrying values between actions. Initially cautioned against a strict state machine. That caution later narrowed to avoiding a generalized graph engine; it did not invalidate the user's emerging ordered-state model.

## E02: Existing-tool exploration and manual execution

**00:51:00Z — User request.** Search for existing tools before designing deeply. Subsequent comparisons discussed Runme, Dagu, Kreya, Bruno, Windmill, and later Kestra and GitHub Actions patterns.

**01:06:42Z — Explicit user clarification.** Bruno was useful but not the intended tool. Dagu and Runme were interesting. The required interaction included running one step, inspecting, changing code, resetting to an earlier useful point, and rerunning. Running the full workflow should also be possible.

## E03: Migration-like navigation

**01:10:23Z — User proposal.** Pair an up action with a down action, and move backward sequentially rather than define every possible stage-to-stage reset path.

**Assistant elaboration.** Proposed a linear chain with optional down actions and a rebuild alternative. Statements implying that only a current-position number was necessary, or that rebuild always established a known state, were too strong. The refined interpretation is proposed in [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).

## E04: Assertion gates

**01:40:15Z — User proposal.** Add assertions that gate progress between stages.

**Assistant elaboration.** Attached checks to the stage rather than only to a transition, making verification reusable after up, down, and rebuild. The exact placement was an assistant proposal, not an explicit user selection. Failure must not be interpreted as proof the previous external state survived.

## E05: Deeper tool comparison

**01:41:16Z and 01:43:04Z — User requests.** Compare the refined idea and investigate existing execution models. The assistant focused on Dagu and Kestra and distinguished execution replay from fixture-state restoration.

This was exploratory research, not an acceptance decision about implementation scope or proof of a market gap. Freshly checked product facts are maintained in [Existing tools](../research/existing-tools.md), rather than preserving all earlier assertions as fact.

## E06: Practical Dagu questions

**01:46:48Z–02:32:44Z — User exploration.** Asked about installation, AI workflow authoring, price/license, drawbacks, CLI/UI usage, and writing an HTTP-then-database inspection workflow. The assistant's examples emphasized Dagu's built-in actions.

No Dagu installation or successful hands-on trial was established in this conversation. Setup commands, release timing, pricing, and AI integration claims are not adopted as Control Tower requirements or maintained installation documentation.

## E07: The authoring boundary

**02:47:34Z — User pushback.** The driver-oriented example felt like the heavyweight opposite of the intended authoring experience, with similarities to a hosted tool or platform.

**03:08:00Z — User clarification.** Compared the YAML to GitHub Actions, suggested simply running executable files via a shebang, and defined the tool's role as buttons to move back and forth and see results while the author owns the work.

**Current correction.** Dagu is not inherently hosted and does not require its database/HTTP drivers. Its script-only path deserves comparison. The user-owned-executable boundary still stands on its own merits; it does not require a false claim about Dagu.

## E08: Parallel tracks and shebang direction

**04:05:56Z — Explicit user decision.** Try Dagu and explore the custom version in parallel. Lead with the shebang as the determining mechanism for now, and investigate storage of inputs and outputs, including alternatives to environment variables.

[ADR-0001](../decisions/0001-user-owned-executables.md) records that authoring direction. It does not turn all later process details into accepted requirements.

## E09: Leading storage candidate

**Assistant proposal after 04:05:56Z.** Separate storage from process transport: workbench-owned state, a read-only structured input view, optional scalar environment values, normal stdout/stderr, and an output file for proposed changes.

**04:26:12Z — User endorsement and research request.** Called this the leading candidate and requested lessons from all previously considered stateful tools, not only Dagu.

**Assistant follow-on proposal.** Suggested `WB_CONTEXT` / `WB_OUTPUT`, persistence across restarts, and per-action state-change history. These exact names, automatic projections, persistence details, and history mechanisms were not subsequently accepted. They are candidates in [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## E10: Documentation authority

**05:03:39Z — Explicit user instruction.** Supplied `christopher-caldwell/control_tower`, requested cohesive documentation under `docs/`, research into documentation organization, front-matter metadata, decision/timeline capture, and direct commits without a PR. Required every design decision to be challenged and evaluated rather than merely transcribed.

The resulting folder structure is a documentation-maintenance choice made within that instruction. The design refinements below remain proposals unless backed by earlier explicit direction.

## Corrections introduced by this documentation review

The [current design audit](../design/current-design.md#every-added-mechanism-has-a-cost) records the detailed challenges. The central corrections are: no inferred rollback after failure; no unconditional rebuild guarantee; no permanent verified label from stale checks; no assumption utilities are harmless; no writable canonical state hidden behind a read-only naming convention; no automatic adoption or destruction of failure outputs; and no assumption that Dagu requires platform drivers.

This is also a provenance correction. Earlier assistant enthusiasm, illustrative YAML, and repeated suggestions are not approval. Accepted user direction, endorsed candidates, and new assistant refinements are now labeled separately.
