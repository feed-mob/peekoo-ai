-- @migrate: alter
-- @tolerates: "duplicate column name"

ALTER TABLE agent_sessions ADD COLUMN acp_session_id TEXT;
