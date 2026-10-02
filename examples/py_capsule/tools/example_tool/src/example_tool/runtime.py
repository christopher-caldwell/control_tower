from __future__ import annotations

from copy import deepcopy
from typing import Any


class _Session:
    def __init__(self, values: dict[str, Any] | None = None):
        self.values = deepcopy(values or {})
        self.events: list[str] = []

    def set_value(self, name: str, value: Any) -> None:
        self.values[name] = value

    def log_event(self, message: str) -> None:
        self.events.append(message)


class _Conversation:
    def __init__(self, metadata: dict[str, Any] | None = None):
        self.metadata = deepcopy(metadata or {})

    def set_metadata(self, name: str, value: Any) -> None:
        self.metadata[name] = value


class ExampleRuntime:
    def __init__(self, context: Any):
        source = context if isinstance(context, dict) else {}
        session = source.get("session", {})
        conversation = source.get("conversation", {})
        self.session = _Session(session.get("values", {}) if isinstance(session, dict) else {})
        self.conversation = _Conversation(
            conversation.get("metadata", {}) if isinstance(conversation, dict) else {}
        )

    def globals(self) -> dict[str, Any]:
        return {"Session": self.session, "Conversation": self.conversation}

    def export(self) -> dict[str, Any]:
        return {
            "session": {
                "values": deepcopy(self.session.values),
                "events": list(self.session.events),
            },
            "conversation": {"metadata": deepcopy(self.conversation.metadata)},
        }
