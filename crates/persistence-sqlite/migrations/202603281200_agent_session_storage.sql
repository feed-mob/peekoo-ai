-- @migrate: create
-- @id: 0014_agent_session_storage
-- @sentinel: agent_sessions

-- Core agent session table (peekoo-managed persistence)
CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    title TEXT,
    status TEXT NOT NULL DEFAULT 'active', -- active, paused, closed
    current_provider TEXT NOT NULL, -- pi-acp, opencode, claude-code, codex, custom
    provider_command TEXT NOT NULL,
    provider_args_json TEXT,
    working_directory TEXT NOT NULL,
    persona_dir TEXT,
    system_prompt TEXT,
    skills_json TEXT,
    provider_state_json TEXT, -- opaque state for resuming with same provider
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_activity_at TEXT,
    closed_at TEXT
);

-- Conversation messages within a session
CREATE TABLE IF NOT EXISTS session_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    sequence_num INTEGER NOT NULL,
    role TEXT NOT NULL, -- system, user, assistant, tool
    content_type TEXT NOT NULL, -- text, tool_call, tool_result, thinking, image
    content_json TEXT NOT NULL,
    tool_name TEXT,
    tool_call_id TEXT,
    provider TEXT,
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_messages_session_id ON session_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_session_messages_sequence ON session_messages(session_id, sequence_num);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_updated ON agent_sessions(updated_at);

-- Tool call results cache
CREATE TABLE IF NOT EXISTS session_tool_results (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    result_json TEXT,
    error_message TEXT,
    executed_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tool_results_session ON session_tool_results(session_id, tool_call_id);
