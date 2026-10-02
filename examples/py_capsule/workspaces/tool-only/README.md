# The smallest shared-tool workspace

This workspace reads explicit input, calls `from example_tool import normalize_record`,
and persists explicit tool values and runtime exports. The tool invokes a real
PyCapsule child with its own environment and injected Session/Conversation runtime.
The final ordinary Python stage observes the value and runtime export.

## Prerequisites and progression

Use built Control Tower binaries, Unix tools, uv / Python 3.12 and access to the pinned PyCapsule Git repository.
Follow [example setup](../../README.md#setup-and-run). This workspace intentionally
uses the example-owned Python project and shared `tools/example_tool`.

```text
001 ordinary Python seed (dateutil) -> data/record.json
002 Python typed tool -> data/tool_result.json and data/runtime_context.json
003 Python inspection -> data/inspection.json
```

## Run forward and backward

From the copied example root after setup:

```sh
workspace=workspaces/tool-only
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 1
cat "$workspace/data/record.json"
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/tool_result.json"
cat "$workspace/data/runtime_context.json"
control-tower up --workspace "$workspace" --stage 3
cat "$workspace/data/inspection.json"
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 2
control-tower down --workspace "$workspace" --stage 1
control-tower down --workspace "$workspace" --stage 0
```

The normalized value is `{"record_id": "123", "label": "EXAMPLE RECORD"}`.
The runtime export records `last_record_id` and `last_tool`; it is an explicit
output, not automatic cross-stage state. Each stage owns only its named outputs.
Reversal retains the checkpoint database, installed dependencies, and capsule logs.

Tool value and runtime export are separate writes. On mutation failure, inspect
partial output before retrying; this deterministic example starts a fresh runtime
and can repeat its tool call. Do not infer safe replay of arbitrary external APIs.
After a verifier failure, repeating the direction retries only that verifier.
See [navigation guidance](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/docs/guides/verification-and-navigation.md)
and the [tool implementation](../../tools/example_tool/README.md).
Return to the [example](../../README.md).
