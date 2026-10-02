# example-tool

The shared [PyCapsule example project](../../README.md) installs this typed editable
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
syncs; strict child validation is tested without flags in ordinary roles.

Both projects resolve `py-capsule` from the same pinned private SSH Git source.
This is development wiring until a released dependency is available; the intended
change preserves stage imports. This facade stays local/editable: its capsule assets
are outside `src/` and its manifest selects its source project. Publishing the tool
would require asset bundling and a child-project installation strategy.
