-- @migrate: alter
-- @id: 0005_task_extensions
-- @tolerates: "duplicate column name"

ALTER TABLE tasks ADD COLUMN assignee TEXT NOT NULL DEFAULT 'user';
ALTER TABLE tasks ADD COLUMN labels_json TEXT NOT NULL DEFAULT '[]';
