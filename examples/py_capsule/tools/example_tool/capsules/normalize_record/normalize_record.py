normalized = {
    "record_id": str(record_id),
    "label": label.strip().upper(),
}

Session.set_value("last_record_id", normalized["record_id"])
Conversation.set_metadata("last_tool", "normalize_record")

return normalized
