-- Add Autopilot (Auto-Advance) settings to pomodoro_state
-- default values match the proposal: work 25, break 5, long 15, int 4, memo 1, auto 0
ALTER TABLE pomodoro_state ADD COLUMN long_break_minutes INTEGER NOT NULL DEFAULT 15;
ALTER TABLE pomodoro_state ADD COLUMN long_break_interval INTEGER NOT NULL DEFAULT 4;
ALTER TABLE pomodoro_state ADD COLUMN auto_advance INTEGER NOT NULL DEFAULT 0;
