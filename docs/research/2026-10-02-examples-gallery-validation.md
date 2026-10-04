---
id: CT-EXAMPLES-GALLERY-VALIDATION
title: Example portability validation
type: research
status: recorded
created: '2026-10-02'
updated: '2026-10-04'
owner: christopher-caldwell
authored_by: assistant
verified_on: '2026-10-02'
method: Source review, real CLI integration, and documentation checks on macOS
sources:
- ../../examples/README.md
- ../../examples/tests/test_ordinary_workflows.py
- ../../examples/tests/test_py_capsule.py
- ../../examples/tests/test_postgres.py
- ../guides/verification-and-navigation.md
---

# Example portability validation

This record covers the example-level portability correction on
`feat/examples-gallery`, starting from pushed revision
`b64ba70d0797943a42f3cadbdee6e290af4b2066`. Baseline inspection and execution
passed 39 core tests and 30 existing integration tests, including PostgreSQL,
with integration temporary directories explicitly redirected outside the checkout.
No Control Tower defect was identified.

## Result and ownership

The portable projects are `simple`, `multi_language`, and `py_capsule`; their
workspaces are scenarios within those examples. Simple owns ordinary dateutil,
psycopg, and Node/dayjs metadata at its root. Its JSON Schema validation stage
retains its closer jsonschema project. Multi-language owns Python/Node metadata
at its root; the Go module, Cargo project/lock, and program sources stay with the
Go/Rust workflow because they define its programs. All moved registry dependency
versions and artifact metadata were preserved.

PyCapsule retains its shared caller projects, typed editable tool, separate child
project, capsule assets, and pinned Git source. The bootstrap completion message
now assumes installed/built CLIs on PATH rather than a Control Tower source tree.
All 132 executable stage roles remain byte-for-byte identical to the baseline.
Core source, CLI interfaces, checkpoint behavior, and recovery semantics are unchanged.

The integration harness copies each complete example with one copy operation,
excluding generated output. Every copy has its README, metadata, sibling
workspaces, and any shared tools. Copies use pytest temporary directories outside
the repository, including paths with spaces and quotes; an assertion rejects
repository-local copy locations. Setup is separate from copying and verifies
locks before and after locked Python sync and Node installation. Stage execution
never imports the repository-owned test helpers.

Documentation teaches whole-example copies and setup at the copied root.
Workspace READMEs focus on progression, explicit handoffs, and recovery. Links
within an example remain relative; links outside it point to the branch's hosted
repository documentation so they remain usable after copying.

## Executed checks

| Check | Outcome |
| --- | --- |
| `cargo build --locked --workspace` | Both CLIs built successfully. |
| `cargo test --locked --workspace` | 39 passed. |
| `./examples/test --postgres` | 31 passed, no skips, in 125.75 seconds. |
| Family selection with `--collect-only` | Simple selects 11 tests, multi-language 3, PyCapsule 17. |
| Runtime-specific setup and opt-in behavior | UUID traversal passes without uv/Node on PATH; Go/Rust skips without toolchains; five selected simple PostgreSQL cases skip without opt-in. |
| Copy isolation guard | Repository-local destination rejected before any copy is made. |
| Example README copy/setup/run commands | All three copied examples completed forward and reverse traversal; PyCapsule bootstrap also passed. |
| Moved dependency lock comparison | Existing registry package versions, hashes, and artifact metadata retained. |
| Setup/traversal lock stability | Successful copied-example setup and workflow traversal leave locks unchanged. |
| Role source, syntax, and permissions | All 132 roles unchanged and executable; Python, Node, and shell syntax passed. |
| Markdown targets and anchors | All 45 Markdown files checked, including hosted links backed by this repository. |
| Whitespace and generated-file review | `git diff --check` clean; installed dependencies and runtime artifacts excluded. |

All eleven workspaces execute through real Control Tower binaries from copied
examples. Coverage retains intermediate handoffs, forward/reverse traversal,
fresh UUIDs, settled-target non-replay, ordinary dependency ownership,
nearest-project schema isolation, stale-lock rejection, typed tool calls, and
real PyCapsule child execution. A new test runs tool-only and node-and-tool
against the same copied shared tool, checks import paths, and proves reversing
one workspace preserves the other's state and outputs.

Node/shell roles and full-stack Node/native psql/shell boundaries run without
Python on PATH and with unusable Python metadata. Both PostgreSQL workflows
retain missing-schema, read-only verification, unrelated-row preservation,
conflicting-row refusal, verifier-only retry, commit-before-receipt failure,
retained-UUID retry, and explicit abandonment coverage. A sentinel proves normal
reversal never calls the unaccepted mutation's `down`. Native SQL string/file
stages retain SQL-error recovery and reverse-verifier retry. Tests use private
PostgreSQL clusters without TCP listeners and ignore application DSNs.

## Correction to the previous validation record

The earlier `b64ba70` restructure recorded 39 core and 30 integration passes,
but incorrectly claimed its default copies had no repository ancestors.
The runner's `--basetemp` was under `examples/test-results/` inside the checkout.
It also copied ordinary workspaces individually and manually assembled partial
PyCapsule copies. Those passes established scenario behavior, not the clarified
whole-example portability contract. This correction replaces those copy boundaries
and removes the repository-local temporary-directory override.

## Limits and retained semantics

These examples teach explicit ownership rather than generic compensation.
Control Tower tracks accepted workflow state. A mutation can commit an external
effect and fail before acceptance; the workflow retains responsibility for that
partial effect. Earlier stages do not clean later unaccepted effects.

The normalization/file examples are deterministic; their successful retries do
not establish safe replay for arbitrary external APIs. Schema setup remains
explicit. Installed environments, build caches, checkpoint databases, and capsule
logs can remain after reversal. PyCapsule still depends on the pinned private Git
source; publishing it is separate work.
