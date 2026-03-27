-- @migrate: create
-- @id: 0002_agent_settings
-- @sentinel: agent_settings

CREATE TABLE agent_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  active_provider_id TEXT NOT NULL,
  active_model_id TEXT NOT NULL,
  system_prompt TEXT,
  max_tool_iterations INTEGER NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);

CREATE TABLE agent_provider_auth (
  provider_id TEXT PRIMARY KEY,
  auth_mode TEXT NOT NULL,
  api_key_ref TEXT,
  oauth_token_ref TEXT,
  oauth_expires_at TEXT,
  oauth_scopes_json TEXT,
  last_error TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE agent_skills (
  skill_id TEXT PRIMARY KEY,
  source_type TEXT NOT NULL,
  path TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);
