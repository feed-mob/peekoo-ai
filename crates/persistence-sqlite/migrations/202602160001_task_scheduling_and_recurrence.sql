-- @migrate: alter
-- @id: 0006_task_scheduling_and_recurrence
-- @tolerates: "duplicate column name"

ALTER TABLE tasks ADD COLUMN scheduled_start_at TEXT;
ALTER TABLE tasks ADD COLUMN scheduled_end_at TEXT;
ALTER TABLE tasks ADD COLUMN estimated_duration_min INTEGER;
ALTER TABLE tasks ADD COLUMN recurrence_rule TEXT;
ALTER TABLE tasks ADD COLUMN parent_task_id TEXT;
