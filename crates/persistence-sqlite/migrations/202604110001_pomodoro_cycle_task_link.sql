-- @migrate: alter
-- @id: 0016_pomodoro_cycle_task_link
-- @tolerates: "duplicate column name"

ALTER TABLE pomodoro_cycle_history ADD COLUMN task_id TEXT;
ALTER TABLE pomodoro_cycle_history ADD COLUMN task_title TEXT;
