---
id: CT-DOCS-VALIDATION
title: User documentation audit and validation
type: research
status: recorded
created: '2026-10-01'
updated: '2026-10-01'
verified_on: '2026-10-01'
owner: christopher-caldwell
authored_by: assistant
method: Pinned-source review, Markdown checks and direct shell-fixture execution
sources:
- ../../README.md
- ../README.md
- ../../examples/simple/uuid-file/README.md
- ../reference/cli.md
- ../reference/stage-executables.md
- documentation-strategy.md
- run-semantics-validation.md
---

# User documentation audit and validation

## Scope and inspected revision

Documentation-only pass against Control Tower `e73e5e6fd7058ac6bb6505114651945ccf5d080d` on October 1, 2026. The source/role contract was read through GitHub, including the actual command parsers, Application movement, Database operations, process/discovery adapters, manifests, fixture and validation records. No Rust source, dependency, SQL schema, test source, or checked-in executable behavior was changed.

The maintained docs were checked for user entry paths, setup prerequisites, executable permissions, workspace-relative paths, environment values, role naming, output behavior, SQLite lifetime, active-transition navigation and consistency with the accepted decisions. Historical design/discovery material remains historical, not an installation guide or additional implementation requirement.

## Changes and observations

The root README now has one source-build/example path using the two built binaries. `just` is optional. The documentation index routes users to setup, authoring, navigation, troubleshooting and two reference pages before the maintainer archive. The example README links all twelve scripts and shows observations at each stage.

The maintained design no longer tells readers that the first implementation has yet to begin. It also no longer suggests that restarting clears SQLite state. Command spellings are taken from the shipped parsers, not the underscore spelling or `steps/` directory used in older illustrations.

The current process handoff is explicitly documented as a **runner-generated UUID**. Earlier discovery describes a stage producing an ID; the implementation does not collect such outputs. ADR-0003 retains that original proposed wording and adds a labeled implementation observation rather than silently changing its authority. No generalized context protocol was introduced to make the example appear more capable.

Reference also distinguishes buffered results from live streaming, normal process persistence from crash recovery, operational database verification from stage verification, and pending verifier retry from rerunning a successful mutation.

## Documentation research used

[GitHub README guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes) informed the short purpose/setup/help entry point and relative links. [GitHub quickstart guidance](https://docs.github.com/en/contributing/style-guide-and-content-model/quickstart-content-type) informed prerequisites, one focused outcome and links to deeper material. [Diátaxis](https://diataxis.fr/start-here/) informed the separation of guides, reference and the existing explanatory archive. The [organization strategy](documentation-strategy.md#october-1-user-documentation-extension) records the project-specific application; no site generator or documentation framework was added.

Rust's official [build](https://doc.rust-lang.org/cargo/commands/cargo-build.html), [install](https://doc.rust-lang.org/cargo/commands/cargo-install.html), [installation](https://rust-lang.org/tools/install/) and [Command::output](https://doc.rust-lang.org/std/process/struct.Command.html#method.output) references were checked for the supporting command/process facts. The repository source remains the authority for Control Tower's interface.

## Newly executed checks in this pass

Environment: Linux x86_64, `/bin/sh` resolving to `dash`, Python 3.13.5. Repository scripts were materialized as byte-identical copies and their Git blob hashes checked against the inspected source; original executable files in the repository were not edited.

| Check | Result and limit |
| --- | --- |
| All twelve shipped role files | Git blob hashes matched; `sh -n` passed for every file. |
| Direct fixture traversal | Executed the actual scripts for `0 -> 1 -> 2 -> 3 -> 2 -> 3 -> 2 -> 1 -> 0`; asserted exact bytes at each stage and absence at baseline. |
| Wrong-state checks | Each of the six supplied verifiers failed on deliberately unsuitable state. Stage 2/verify-down additionally rejected an absent file rather than accepting it as empty. |
| Stage 3 reversal | Confirmed suffix removal preserves an arbitrary preceding value; missing suffix failed without changing contents. |
| Script execution total | 25 shipped-script invocations: 17 expected successes and 8 expected failures, all matching assertions. |
| New workspace-authoring guide | Executed the guide's literal heredoc creation block, then all four generated role scripts; observed `ready` and then marker absence. |
| Documentation structure | Checked changed Markdown for parseable front matter, unique IDs, balanced fences, relative link targets, linked headings and shell-block syntax. Source-file links were matched to the inspected repository paths. |
| Command/source alignment | Checked CLI/Cargo package and binary names, flag vs positional interfaces, setup order and environment names against source. This is source inspection, not executable verification. |

The direct script harness supplied one UUID and the documented environment, with the working directory set to each role's stage directory. It did **not** execute the Rust runner, its state machine, its SQLite adapter, or CLI output rendering. It proves the shipped shell fixture's behavior, not an independent end-to-end Control Tower acceptance run.

## Checks not executed here

This environment had no `cargo`, `rustc` or `just`. A direct Git checkout attempt also failed because the shell could not resolve GitHub; the GitHub connector remained available for source inspection and publishing. Therefore this pass did not run `cargo build`, Cargo install, `cargo fmt`, Clippy, the workspace Rust tests, the literal full README quickstart, or the CLI failure walkthrough end-to-end. They are **NOT EXERCISED in this documentation pass**, not reported as green.

The existing [run-semantics validation](run-semantics-validation.md#executed-verification) records the worker's separate macOS/Rust 1.94.0 run: 31 passing tests and manual real-CLI/SQLite fixture exercises. That evidence is preserved and linked, not relabeled as an independent run by this documentation reviewer. Neither it nor the script-only Linux check verifies native Windows or the manifest's declared Rust 1.85 minimum.

## Reproduce full documentation acceptance locally

From the checkout root, follow the root README exactly in a fresh workspace. Then execute the [stage-by-stage example](../../examples/simple/uuid-file/README.md#walk-forward-and-observe) and [failed-verifier exercise](../guides/verification-and-navigation.md#try-a-verification-failure). Those instructions show expected outcomes and identify intentional nonzero exits.

Run the workspace checks:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --locked --workspace
git diff --check
```

Record the actual revision, toolchain and results when these are rerun. Keep future execution evidence separate from this pass's limitations.
