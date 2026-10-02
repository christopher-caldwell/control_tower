---
id: CT-QUESTIONS
title: Open questions and validation
type: design
status: maintained
created: '2026-09-30'
updated: '2026-10-02'
owner: christopher-caldwell
authored_by: assistant
sources:
- current-design.md
- ../research/discovery-01.md
- ../research/playbook-compliance.md
- ../research/run-semantics-validation.md
- ../research/2026-10-01-guided-usage-findings.md
- ../research/2026-10-01-core-v0-completion.md
- ../decisions/0002-stage-navigation-and-verification.md
- ../decisions/0003-session-state-and-process-io.md
---

# Open questions and validation

The first implementation and its architecture/semantics corrections exist. This is not a setup checklist or a list of blockers before using the example. Start with [the user guide](../guides/getting-started.md).

**Core v0 is complete as an owner-use candidate.** The [completion record](../research/2026-10-01-core-v0-completion.md) defines the bounded standard, fresh and inherited evidence, and remaining limits. These questions guide later real use; they do not reopen that milestone.

## Implemented experiments

The four-role filesystem convention, SQLite across CLI invocations, verifier retry without mutation replay, and same-stage directional reversal have [executed implementation evidence](../research/run-semantics-validation.md). The [playbook ledger](../research/playbook-compliance.md) preserves the preceding architecture assessment. Keep their revision, platform and test limits when citing those records.

The original example uses one shared **runner-generated** UUID. The optional [generated-ID example](../../examples/simple/generated-id/README.md) adds an author-owned SQLite/JSON handoff without adding managed context. The distinction is explicit in [the process reference](../reference/stage-executables.md#what-the-uuid-does-and-does-not-mean).

The [guided-usage findings register](../research/2026-10-01-guided-usage-findings.md) records the core feedback/checkpoint refinements, new acceptance evidence, and every F01–F15 disposition with corrections. Deferred findings are retained knowledge and future discussion inputs, **not an implementation backlog or additional acceptance criteria for this round**. Their revisit triggers do not authorize capabilities by themselves.

## Questions for actual use

Filesystem authoring, application-generated IDs in author-owned files, and the application edit/backout/rerun loop have practical evidence in AR06/AR07 and the completion record. Farther pending continuation and layered compensation have fresh AR08 evidence. The remaining question is convenience across repeated actual owner tickets: where do authoring, setup, inspection or handoff costs recur enough to justify a small change?

Answer these from use. They do not authorize a config language, output-patch system, extra action type or additional frontend before a real need appears.

## Deferred edge cases

When an up/down executable itself exits nonzero, v0 reports failure and stops automatic movement. Richer cleanup/recovery for partially effective failed mutations remains deferred; it must not be confused with the already implemented reversal after a successful mutation's verifier fails.

Also deferred: Rust-process crash reconciliation, directory-structure drift protection, concurrent instances, workspace-wide reset semantics, Tauri/HTTP, helper ecosystems and generalized context/value semantics. SQLite persistence does not bring those features into scope.

Historical starting points are retained in the [discovery brief](discovery-brief.md) and [discovery record](../research/discovery-01.md). They should not be reissued as current implementation tasks without new evidence.
