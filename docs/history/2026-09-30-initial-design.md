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
  end: '2026-09-30T19:46:29Z'
source_kind: Current conversation with relevant history recovery
---

# Initial design discussion and corrections

## Scope and provenance

This is a curated reconstruction of the discussion, not a verbatim transcript. Times below are user-message timestamps in UTC on September 30, 2026.

CT-DISCUSSION-2026-09-30 is a correlation label created for these docs. It is not a recovered ChatGPT conversation UUID.

The repository was originally inspected on main at initial commit 29d1446806b7127c0e0bca471aa899c6021e2f46 with only the short root README. The owner authorized cohesive documentation under docs/ and direct commits without a PR. Work-specific identifiers and actual data are intentionally replaced with generic fixture examples because the repository is public.

This record documents decision history, not a promised development schedule.

## E01: Initial problem

**00:49:35Z — User direction.** A personal, local, Rust-based app, native or web, should expose configured test actions as buttons. The example was a test user with associated data that must be created, mutated, inspected, and eventually recreated because of business constraints. Current work was scattered between SQL snippets and terminal commands. Application source editing was outside the tool's role.

**Assistant interpretation.** Suggested a developer workbench rather than another HTTP client and emphasized carrying values between actions.

## E02: Existing-tool exploration and manual execution

**00:51:00Z — User request.** Search for existing tools before designing deeply. Comparisons discussed Runme, Dagu, Kreya, Bruno, Windmill, Kestra, and GitHub Actions patterns.

**01:06:42Z — Explicit user clarification.** Bruno was useful but not the intended tool. Dagu and Runme were interesting. The required interaction included running one step, inspecting, changing code, resetting to an earlier useful point, and rerunning. Running the full workflow should also be possible.

## E03: Migration-like navigation

**01:10:23Z — User proposal.** Pair an up action with a down action and move backward sequentially rather than define every possible pairwise reset path.

This became the foundation of [ADR-0002](../decisions/0002-stage-navigation-and-verification.md).

## E04: Assertion gates

**01:40:15Z — User proposal.** Add assertions as gates between useful positions, illustrated as 0 -> 1 -> 2 [must meet criteria x,y] -> 3.

The exact timing of that gate remained open.

## E05: Deeper tool comparison

**01:41:16Z and 01:43:04Z — User requests.** Compare the refined idea and investigate existing execution models. The assistant focused on Dagu and Kestra and distinguished execution replay from fixture-state restoration.

This was exploratory research, not proof that a market gap existed.

## E06: Practical Dagu questions

**01:46:48Z–02:32:44Z — User exploration.** Asked about Dagu installation, AI workflow authoring, price/license, drawbacks, CLI/UI usage, and authoring an HTTP-then-database inspection workflow.

## E07: The authoring boundary

**02:47:34Z — User pushback.** The driver-oriented Dagu example felt like a heavyweight opposite of the intended authoring experience.

**03:08:00Z — User clarification.** Compared Dagu YAML to GitHub Actions, suggested running executable files via a shebang, and defined the tool's role as buttons to move back and forth and see results while the author owns the work.

**Correction retained.** Dagu does not require its HTTP/database drivers. The user-owned-executable boundary stands on its own merits rather than on a false limitation of Dagu.

## E08: Parallel tracks and shebang direction

**04:05:56Z — Explicit user decision.** Try Dagu while exploring the custom version in parallel. Lead with the shebang as the determining mechanism and investigate storage of inputs and outputs.

[ADR-0001](../decisions/0001-user-owned-executables.md) records that authoring direction.

## E09: Leading storage candidate

The assistant proposed separating durable context from process transport: a workbench-owned state/context view, normal stdout/stderr, and a separate output file for machine updates.

**04:26:12Z — User endorsement and research request.** Called this the leading candidate and requested lessons from all previously considered stateful tools.

The exact names, encoding, persistence backend, and environment projection remained unaccepted details in [ADR-0003](../decisions/0003-session-state-and-process-io.md).

## E10: Documentation authority

**05:03:39Z — Explicit user instruction.** Supplied christopher-caldwell/control_tower, requested cohesive documentation under docs/, research into documentation organization, front-matter metadata, decision/timeline capture, and direct commits without a PR. Required every design decision to be challenged rather than merely transcribed.

## E11: Dagu trial

**18:20:47Z — User request.** Asked for a ready-to-run Dagu starter pack.

**18:57:25Z–19:01:55Z — Hands-on reaction.** After running Dagu locally and encountering the local account/setup flow, the user concluded that Dagu was substantially heavier than the original idea, while still potentially useful.

This changed the comparison from theoretical product research to direct product-fit evidence. Dagu remained a source of patterns, not the target architecture.

## E12: Core identity settled

**19:13:17Z — User clarification.** Dagu was close but not the intended product, and building the custom version would be worthwhile and enjoyable. The user explicitly compared Control Tower to a database migration runner: up/down correctness is the author's responsibility; the tool just runs the authored operations.

**19:19:28Z — Explicit product boundary.** The machinery should be dumb. Control Tower is “a migration runner for whatever you want to do,” with no login or hosting and no intention to become an orchestration product. It is a workbench. Sidecar helpers for common tasks may exist later but are not core.

**19:42:33Z — Helper clarification.** A possible Node/TypeScript helper would be consumer-facing convenience, just like a future Postgres helper, and still sit outside the Rust execution model.

These statements simplify [ADR-0002](../decisions/0002-stage-navigation-and-verification.md) and narrow the current design.

## E13: Three-step design probe

**19:46:29Z — User request.** Work through a three-step example and document findings according to the agreed structure.

The resulting [Three-step workspace design probe](../design/three-step-workspace.md) tests create-user, create-associated-record, and mutate-record as ordered steps with up/down executables plus an auxiliary inspect action.

The probe finds that a convention-only directory can express the minimum example, that context needs assignment and removal, and that assertion timing is the first unresolved semantic choice that materially affects the runtime contract.

## Corrections introduced over the discussion

Several earlier assistant ideas were intentionally reduced:

- No generalized workflow/DAG model is needed.
- No “uncertain external state” engine is required; a failed attempt plus unchanged recorded position is sufficient bookkeeping.
- Down is author-provided compensation, not guaranteed inverse execution.
- Rebuild is not a promised primitive.
- Central YAML/TOML configuration is no longer assumed.
- Rich structured context is not justified by the first example; scalar IDs plus removal are enough to begin.
- Dagu's broader scope is a product-fit distinction, not evidence that it cannot run ordinary scripts.

Accepted user direction, endorsed candidates, and unresolved assistant proposals remain labeled separately.
