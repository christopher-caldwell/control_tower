---
id: ADR-0002
title: Navigate an ordered fixture with explicit compensation and checks
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: pending-user-review-of-semantics
sources:
- ../history/2026-09-30-initial-design.md#e03-migration-like-navigation
- ../history/2026-09-30-initial-design.md#e04-assertion-gates
---

# ADR-0002: Navigate an ordered fixture with explicit compensation and checks

## Standing

The user proposed migration-like up/down actions and assertion gates. This record proposes a precise interpretation and corrects gaps in the early assistant explanations. **The complete semantics below have not been accepted.**

## Proposed model

Start with an ordered chain rather than a generalized dependency graph:

```text
stage 0 <--down 1-- stage 1 <--down 2-- stage 2 <--down 3-- stage 3
        ---up 1-->         ---up 2-->         ---up 3-->
```

`up N` attempts to establish stage N from stage N-1. `down N` attempts to reestablish stage N-1 from stage N. Moving from 6 to 2 therefore invokes down 6, down 5, down 4, and down 3. Moving from 2 to 5 invokes up 3, up 4, and up 5.

This removes the need to author every pairwise transition. It does not remove the obligation to handle partial execution or external changes.

## Verification: useful evidence, not a proof of the world

Attach reusable checks to the destination stage as the leading interpretation. They can be used after forward movement, backward movement, rebuilding, or a manual Verify action. A stage check might verify that the fixture user exists, its associated record belongs to that user, and the record is open.

The user's original gates could instead be transition-specific preconditions. Stage checks are attractive because they can be reused regardless of arrival direction, but that placement is still a proposal. Verification coverage may be optional; an unchecked stage must not be labeled verified.

Passing configured checks only establishes those observations at that time. It does not prove complete equality of external state, establish a unique stage among overlapping checks, or guarantee the state remains unchanged. Reverification after interruption, out-of-band mutation, or reopening a session is therefore a candidate rule.

## The failure correction

Suppose up 3 closes a record and then verification fails. The fixture is not safely “still at stage 2.” The previous verified checkpoint is historical information; the present condition is uncertain.

The proposed UI should distinguish at least what was last verified, what transition was attempted, and whether the current condition is known. These are conceptual distinctions, not a required database schema. A failed action, failed check, or interrupted transition stops automatic navigation. Recovery may involve inspection, an authored repair, compensation, or rebuilding. Do not automatically repeat a mutation merely because its result was not accepted.

For navigation over multiple stages, propose verifying each reached stage before continuing. Preflight the requested path for missing actions before causing side effects, while acknowledging that executable presence cannot prove reversibility. These operational rules still need the [Q3 exercise](../design/open-questions.md#q3--what-does-navigation-promise).

## Down and rebuild are different operations

A down action is authored compensation. Reopening a record may restore the conditions needed for a test while leaving audit history, emitted messages, or other side effects intact. A transition may legitimately have no down action.

A missing down action is not an invitation to decrement a stage counter. Make the unavailable path visible without silently substituting another operation. If a real down action fails partway, stop and mark uncertainty; do not continue down the chain under a false assumption.

Rebuild means running an explicit recipe that establishes a new useful fixture and verifies its target. It may include cleanup or intentionally leave an old fixture for separate cleanup. Discarding context is neither database cleanup nor a guaranteed reset. Preserve available old identifiers until the chosen cleanup policy permits discarding them.

## Individual actions and full runs

An inspection can run without moving the stage, provided it does not intentionally alter the fixture. A general utility that changes fixture data or identifying context cannot automatically preserve the verified label. The authoring distinction and invalidation rule are unresolved.

“Continue to target” and “start fresh and run all” should have distinct meanings. The former attempts the remaining transitions; the latter requires a defined fresh-fixture recipe. Exact labels and whether both belong in the first UI remain open. Rerunning up 3 from stage 3 is also not automatically safe: first establish its source condition or use an explicitly repeatable action.

## Alternatives, costs, and validation

A loose action pad is simpler, but leaves the reset sequence in the developer's memory. A generalized graph offers branching, but introduces ambiguous backward paths without a demonstrated need. Snapshot-based database restoration can be useful when the author controls the environment, but is not a generic solution for external API effects.

The linear model earns its place only if one real recipe benefits from it. Try a forward run, an intermediate rollback, a missing reverse action, failed compensation, overlapping checks, and an external mutation. If those require extensive controller machinery, simplify the promise rather than hiding it behind a stage number.
