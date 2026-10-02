from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypedDict, cast

from py_capsule import run


class NormalizedRecord(TypedDict):
    record_id: str
    label: str


@dataclass(frozen=True)
class NormalizeRecordResult:
    value: NormalizedRecord
    runtime_export: dict[str, Any]


_CAPSULE = Path(__file__).resolve().parents[2] / "capsules" / "normalize_record"


def normalize_record(
    *,
    record_id: str,
    label: str,
    runtime_context: dict[str, Any] | None = None,
) -> NormalizeRecordResult:
    result = run(
        _CAPSULE,
        inputs={"record_id": record_id, "label": label},
        runtime="example_tool.runtime:ExampleRuntime",
        runtime_context=runtime_context or {},
    )

    if not isinstance(result.value, dict):
        raise RuntimeError("normalize_record returned a non-object result")
    if not result.has_runtime_export or not isinstance(result.runtime_export, dict):
        raise RuntimeError("normalize_record did not export runtime state")

    return NormalizeRecordResult(
        value=cast(NormalizedRecord, result.value),
        runtime_export=result.runtime_export,
    )
