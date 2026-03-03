CREATE TABLE agent_provider_configs (
  provider_id TEXT PRIMARY KEY,
  base_url TEXT NOT NULL,
  api TEXT NOT NULL,
  auth_header INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);
