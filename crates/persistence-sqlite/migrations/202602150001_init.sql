-- @migrate: create
-- @id: 0001_init
-- @sentinel: tasks

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  notes TEXT,
  status TEXT NOT NULL,
  priority TEXT NOT NULL,
  due_at TEXT,
  source TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE task_events (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE pomodoro_sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  duration_sec INTEGER NOT NULL,
  interruptions INTEGER NOT NULL DEFAULT 0,
  notes TEXT
);

CREATE TABLE conversations (
  id TEXT PRIMARY KEY,
  title TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE calendar_accounts (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  external_account_id TEXT,
  scopes_json TEXT NOT NULL,
  token_ref TEXT NOT NULL,
  token_expires_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE calendar_events (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  external_event_id TEXT NOT NULL,
  title TEXT NOT NULL,
  start_at TEXT NOT NULL,
  end_at TEXT NOT NULL,
  all_day INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  etag TEXT,
  updated_at_remote TEXT,
  updated_at_local TEXT NOT NULL
);

CREATE TABLE calendar_sync_state (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  sync_token TEXT,
  last_sync_at TEXT,
  error_count INTEGER NOT NULL DEFAULT 0,
  last_error TEXT
);

CREATE TABLE plugins (
  id TEXT PRIMARY KEY,
  plugin_key TEXT NOT NULL,
  version TEXT NOT NULL,
  plugin_type TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  manifest_json TEXT NOT NULL,
  installed_at TEXT NOT NULL
);

CREATE TABLE plugin_permissions (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  capability TEXT NOT NULL,
  granted INTEGER NOT NULL
);

CREATE TABLE plugin_state (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  state_key TEXT NOT NULL,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE event_log (
  id TEXT PRIMARY KEY,
  trace_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
