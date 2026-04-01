-- @migrate: alter
-- @tolerates: "duplicate column name"

ALTER TABLE agent_sessions ADD COLUMN session_type TEXT NOT NULL DEFAULT 'chat';
CREATE INDEX IF NOT EXISTS idx_agent_sessions_type ON agent_sessions(session_type);
