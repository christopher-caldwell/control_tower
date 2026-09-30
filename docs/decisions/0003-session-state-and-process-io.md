---
id: ADR-0003
title: Separate completed context, active-transition context, and process I/O
type: decision
status: proposed
created: '2026-09-30'
updated: '2026-09-30'
owner: christopher-caldwell
authored_by: assistant
decision_authority: user-endorsed-candidate-with-unapproved-details
sources:
- ../history/2026-09-30-initial-design.md#e09-leading-storage-candidate
- ../history/2026-09-30-initial-design.md#e17-directional-verification-proposal
- ../research/existing-tools.md
- ../design/three-step-workspace.md
---

# ADR-0003: Separate completed context, active-transition context, and process I/O

## Standing

The general context/output separation remains the leading candidate.

The possible verify-down hook makes one refinement useful: “in-progress context” should be understood as **active-transition context**, because a transition can now be moving either up or down.

The exact persistence and wire formats remain undecided.

## Logical context layers

- **completed snapshot** — the context associated with the last accepted completed position;
- **active-transition working values** — values needed while an up or down mutation awaits directional verification;
- **effective context** — the view supplied to the current transition's verifier and relevant inspection actions.

The process boundary still points toward:

~~~text
effective context -> executable
stdout/stderr -> human-visible result
machine output -> proposed/working context changes
exit status -> operation result
~~~

## Forward example

~~~text
completed 02 snapshot:
  user_id
  record_id

03/up produces:
  mutation_id

03/verify-up fails

effective context:
  user_id
  record_id
  mutation_id
~~~

If verify-up later passes, the resulting effective context can become the completed-03 snapshot.

## Backward example

Suppose completed 02 contains record_id.

02/down may delete the external record, but 02/verify-down may still need record_id to prove the deletion happened.

Therefore do not remove record_id from the verifier's context merely because down exited zero.

After verify-down succeeds, the workbench needs a context appropriate to completed 01.

## Snapshot restoration candidate

One attractive mechanism is to save the context snapshot at each completed position.

Then:

~~~text
completed 01 snapshot:
  user_id

completed 02 snapshot:
  user_id
  record_id
~~~

A successful verified 02/down can simply restore the saved 01 snapshot.

Advantages:

- down scripts do not need boilerplate to unset every value introduced by up;
- verify-down can still see the source-step identifiers during verification;
- up/down navigation naturally mirrors the migration stack.

Costs:

- snapshots consume some local storage, though expected context is tiny;
- if down intentionally establishes a different context than the historical 01 snapshot, the model needs an override mechanism or explicit output policy;
- changing migration definitions may invalidate saved snapshots.

This candidate now deserves comparison against explicit down-published set/unset changes.

## Explicit down-output candidate

Alternatively, 02/down can publish context changes itself.

That is maximally explicit but makes the action author responsible for external compensation **and** Control Tower bookkeeping cleanup.

It may also require verify-down to see a pre-commit effective view that preserves identifiers the down output intends to remove.

## Failed mutation output

An up or down can emit machine output and later exit nonzero. The policy for those values remains open.

That is separate from a mutation exiting zero followed by failed verification; in the latter case active-transition context clearly has a role.

## Verify output

No current example requires verify-up or verify-down to publish Control Tower context.

The leading simplification is for both verifiers to be read-only at the context boundary: read effective context, print useful output, and communicate pass/fail via exit status.

## Next validation

Implement the conceptual 01 -> 02 -> 01 example on paper using both:

1. saved completed snapshots,
2. explicit down set/unset output.

Choose the mechanism that keeps authoring small while preserving enough context for directional verification.
