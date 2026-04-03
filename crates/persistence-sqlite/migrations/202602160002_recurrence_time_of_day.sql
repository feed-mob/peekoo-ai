-- @migrate: alter
-- @id: 0007_recurrence_time_of_day
-- @tolerates: "duplicate column name"

ALTER TABLE tasks ADD COLUMN recurrence_time_of_day TEXT;
