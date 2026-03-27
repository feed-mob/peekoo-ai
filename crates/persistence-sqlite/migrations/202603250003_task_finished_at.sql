-- @migrate: alter
-- @id: 0011_task_finished_at
-- @tolerates: "duplicate column name"

ALTER TABLE tasks ADD COLUMN finished_at TEXT;

UPDATE tasks
SET finished_at = updated_at
WHERE status = 'done'
  AND finished_at IS NULL;
