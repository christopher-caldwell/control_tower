INSERT INTO workbench_state
                    (id, completed_stage_count, uuid, pending_stage_index, pending_direction)
                 VALUES (1, ?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                    completed_stage_count = excluded.completed_stage_count,
                    uuid = excluded.uuid,
                    pending_stage_index = excluded.pending_stage_index,
                    pending_direction = excluded.pending_direction;
