-- @migrate: alter
-- @id: 0012_pomo_memo_v1
-- @tolerates: "duplicate column name"

-- Add memo column to pomodoro_cycle_history
ALTER TABLE pomodoro_cycle_history ADD COLUMN memo TEXT;
