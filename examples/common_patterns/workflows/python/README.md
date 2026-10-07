# Python shared-pattern Workflow

This two-stage scenario captures a generated value in stage 1 and reads and
checks that saved value in stage 2. Both Stage Actions import the reusable
helpers from the Workspace's `lib/python/` directory.

The helpers handle saved values and process results. Comparisons use the runtime's
`unittest.TestCase` assertions directly; use your project's existing assertion and
normalization tools for more involved checks.

From the Workspace root, prepare this Workflow with the commands in the parent
[README](../../README.md#copy-and-run), then run `control-tower up --workflow
workflows/python --stage 2`. Inspect `workflows/python/data/state.json`; reset
with `control-tower down --workflow workflows/python --stage 0`.
