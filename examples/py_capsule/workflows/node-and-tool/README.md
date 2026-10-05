# Node and an encapsulated Python tool

This workflow reads explicit input, calls `from example_tool import normalize_record`,
and persists explicit tool values and runtime exports. The tool invokes a real
PyCapsule child with its own environment and injected Session/Conversation runtime.
Node stages on either side of the Python call show that shared Python
capabilities can be consumed in an ordinary multi-runtime investigation.

## Prerequisites and progression

Use the built `control-tower` CLI, Unix tools, uv / Python 3.12, Node / npm, and access to the pinned PyCapsule Git repository.
Follow [example setup](../../README.md#setup-and-run). This workflow intentionally
uses the example-owned Python project and shared `tools/example_tool`.

```text
001 Node seed (dayjs UTC) -> data/record.json
002 Python typed tool -> data/tool_result.json and data/runtime_context.json
003 Node inspection (dayjs UTC) -> data/node_inspection.json
```

## Run forward and backward

From the copied example root after setup:

```sh
workflow=workflows/node-and-tool
control-tower db bootstrap-local "$workflow"
control-tower db migrate-local "$workflow"
control-tower db verify-local "$workflow"
control-tower up --workflow "$workflow" --stage 1
cat "$workflow/data/record.json"
control-tower up --workflow "$workflow" --stage 2
cat "$workflow/data/tool_result.json"
cat "$workflow/data/runtime_context.json"
control-tower up --workflow "$workflow" --stage 3
cat "$workflow/data/node_inspection.json"
control-tower status --workflow "$workflow"
control-tower down --workflow "$workflow" --stage 2
control-tower down --workflow "$workflow" --stage 1
control-tower down --workflow "$workflow" --stage 0
```

The normalized value is `{"record_id": "123", "label": "EXAMPLE RECORD"}`.
The runtime export records `last_record_id` and `last_tool`; it is an explicit
output, not automatic cross-stage state. Each stage owns only its named outputs.
Reversal retains the checkpoint database, installed dependencies, and capsule logs.

Tool value and runtime export are separate writes. On mutation failure, inspect
partial output before retrying; this deterministic example starts a fresh runtime
and can repeat its tool call. Do not infer safe replay of arbitrary external APIs.
After a verifier failure, repeating the direction retries only that verifier.
See [navigation guidance](https://github.com/christopher-caldwell/control_tower/blob/main/docs/guides/verification-and-navigation.md)
and the [tool implementation](../../tools/example_tool/README.md).
Return to the [example](../../README.md).
