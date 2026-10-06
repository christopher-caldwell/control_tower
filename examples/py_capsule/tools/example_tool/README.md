# example-tool

The [PyCapsule example](../../README.md)'s caller Python project installs this typed editable
API for ordinary stages:

```python
from example_tool import normalize_record
result = normalize_record(record_id="123", label=" example record ")
```

`result.value` is a typed normalized record; `result.runtime_export` is the
JSON-compatible Session/Conversation export. Stages explicitly read their inputs
and persist these values. This tool owns the capsule manifest/body, injected
runtime, PyCapsule invocation, and tool-specific dependencies. It owns no scenario
filesystem handoff or stage mapping.

The manifest selects this project's child `.venv`, separate from the caller's
shared environment. Declare body/runtime requirements here, then update
both the tool and example locks. Category `bootstrap` validates both with locked
syncs; strict child validation is tested without flags in ordinary Stage Actions.

Both projects install the published `capsule-runner==0.0.1` distribution from
PyPI. It provides the `py_capsule` import used by this facade; stages continue
to import `example_tool`. No private Git access is needed. This facade stays
local/editable: its capsule assets are outside `src/` and its manifest selects its source project. Publishing the tool
would require asset bundling and a child-project installation strategy.
