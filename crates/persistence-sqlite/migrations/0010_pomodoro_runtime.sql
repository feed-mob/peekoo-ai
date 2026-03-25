CREATE TABLE IF NOT EXISTS pomodoro_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  mode TEXT NOT NULL,
  state TEXT NOT NULL,
  minutes INTEGER NOT NULL,
  time_remaining_secs INTEGER NOT NULL,
  started_at_epoch INTEGER,
  expected_fire_at_epoch INTEGER,
  default_work_minutes INTEGER NOT NULL,
  default_break_minutes INTEGER NOT NULL,
  enable_memo INTEGER NOT NULL DEFAULT 0,
  completed_focus INTEGER NOT NULL DEFAULT 0,
  completed_breaks INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO pomodoro_state (
  id,
  mode,
  state,
  minutes,
  time_remaining_secs,
  started_at_epoch,
  expected_fire_at_epoch,
  default_work_minutes,
  default_break_minutes,
  enable_memo,
  completed_focus,
  completed_breaks,
  updated_at
) VALUES (
  1,
  'work',
  'Idle',
  25,
  1500,
  NULL,
  NULL,
  25,
  5,
  0,
  0,
  0,
  datetime('now')
);

CREATE TABLE IF NOT EXISTS pomodoro_cycle_history (
  id TEXT PRIMARY KEY,
  mode TEXT NOT NULL,
  planned_minutes INTEGER NOT NULL,
  actual_elapsed_secs INTEGER NOT NULL,
  outcome TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  memo_requested INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_pomodoro_cycle_history_ended_at
ON pomodoro_cycle_history(ended_at DESC);
