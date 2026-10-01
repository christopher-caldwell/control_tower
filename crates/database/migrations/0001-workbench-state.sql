CREATE TABLE IF NOT EXISTS workbench_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    completed_stage_count INTEGER NOT NULL,
    uuid TEXT,
    pending_stage_index INTEGER,
    pending_direction TEXT
);
CREATE TABLE control_tower_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);
INSERT INTO control_tower_migrations VALUES (1, 'workbench-state');
PRAGMA user_version = 1;
