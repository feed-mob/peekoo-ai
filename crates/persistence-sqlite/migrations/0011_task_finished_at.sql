ALTER TABLE tasks ADD COLUMN finished_at TEXT;

UPDATE tasks
SET finished_at = updated_at
WHERE status = 'done'
  AND finished_at IS NULL;
