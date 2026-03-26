-- @migrate: alter
-- @id: 0013_pomo_daily_reset_v1
-- @tolerates: "duplicate column name"

-- Add last_reset_date column to track daily counter resets
ALTER TABLE pomodoro_state 
ADD COLUMN last_reset_date TEXT;

-- Initialize with current date for existing row
UPDATE pomodoro_state 
SET last_reset_date = date('now') 
WHERE id = 1;
