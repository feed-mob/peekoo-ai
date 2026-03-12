CREATE TABLE IF NOT EXISTS plugins (
  id TEXT PRIMARY KEY,
  plugin_key TEXT NOT NULL,
  version TEXT NOT NULL,
  plugin_type TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  manifest_json TEXT NOT NULL,
  installed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_permissions (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  capability TEXT NOT NULL,
  granted INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_state (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  state_key TEXT NOT NULL,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS event_log (
  id TEXT PRIMARY KEY,
  trace_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
