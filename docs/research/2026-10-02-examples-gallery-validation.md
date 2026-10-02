---
id: CT-EXAMPLES-GALLERY-VALIDATION
title: Example family and workspace validation
type: research
status: recorded
created: '2026-10-02'
updated: '2026-10-02'
owner: christopher-caldwell
authored_by: assistant
verified_on: '2026-10-02'
method: Source review, real CLI integration, and documentation checks on macOS
sources:
- ../../examples/README.md
- ../../examples/tests/test_ordinary_workspaces.py
- ../../examples/tests/test_py_capsule.py
- ../../examples/tests/test_postgres.py
- ../guides/verification-and-navigation.md
---

# Example family and workspace validation

This record accompanies the family/workspace restructure on `feat/examples-gallery`,
starting from pushed revision `4bc32d42fe754731d99d56a73424b6e64a22aadb`.
The baseline passed 39 core tests and 17 existing PyCapsule/gallery tests, including
PostgreSQL. No blocking Control Tower defect was identified.

## Result and ownership

The gallery now has six simple workspaces, two ordinary multi-language workspaces,
and three PyCapsule workspaces. Every scenario lives under its family's
`workspaces/` directory. Simple and multi-language projects own dependencies inside
each workspace. Only PyCapsule owns a shared runtime project and `tools/` layer.
The optional test harness owns separate test dependencies and is never imported
by stage files.

All 21 existing UUID/generated-ID executable and support files were compared with
the baseline and preserved byte-for-byte. The only Rust edit changes the UUID
integration fixture path. The typed tool API, PyCapsule child project, and pinned
Git dependency remain intact. No core runtime, checkpoint, rollback, or CLI
interface behavior was changed.

## Executed checks

| Check | Outcome |
| --- | --- |
| `cargo test --locked --workspace` | 39 passed. |
| `./examples/test --postgres` | 30 passed, no skips, in 116.40 seconds. |
| Family selection with `--collect-only` | Simple selects 11 tests, multi-language 3, PyCapsule 16. |
| Simple PostgreSQL selection without `--postgres` | All 5 database tests skip explicitly; no server fixture runs. |
| `--postgres` with server tools removed from PATH | Clear prerequisite error before test execution. |
| Go/Rust selection with toolchains removed from PATH | Explicit optional-toolchain skip. |
| Shell/Python/Node syntax and role permissions | All 132 role files checked. |
| Relative Markdown files and heading anchors | All 44 then-existing Markdown files resolved. |
| Staged whitespace and generated-file review | Clean; no runtime outputs or installed dependencies staged. |

The integration suite uses this checkout's real CLIs against disposable copies in
paths containing spaces and quotes. Ordinary copies have no repository or family
runtime project above them. PyCapsule copies retain the documented shared projects
and sibling tool. All eleven workspaces execute successfully, including the Go
and Rust programs compiled by ordinary `go run` / `cargo run` stage commands.

Coverage includes intermediate handoffs, forward/reverse traversal, new UUIDs,
settled-target non-replay, dependency ownership, unchanged locks, nearest-project
schema isolation, typed shared-tool calls, and actual PyCapsule child execution.
Node/shell roles and the full-stack Node/native psql/shell boundaries run without
Python on PATH and with unusable Python project metadata.

Both PostgreSQL workspaces exercise missing schema, read-only database checks,
unrelated-row preservation, conflicting-row refusal, verifier-only retry, and
commit-before-receipt failure. Retry completes the existing run-owned operation.
Abandonment explicitly removes only the retained UUID's row before returning to
baseline; a sentinel proves the unaccepted mutation's `down` is not invoked.
Native SQL string/file stages also exercise SQL-error recovery and reverse-verifier
retry. Tests start private PostgreSQL clusters with no TCP listeners and ignore
application DSNs.

An initial gallery run passed 28 checks and exposed one missing import in a
migrated test. The test now uses the common native-environment helper; the complete
30-test rerun above includes that correction and the additional full-stack boundary
check. This was a test-harness error, not a core runtime defect.

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
