-- @migrate: create
-- @id: 0014_acp_runtime_v2
-- @sentinel: agent_sessions
--
-- Consolidated migration for ACP runtime architecture.
-- Replaces: 0014_session_storage, 0015_provider_configs, 0016_runtime_architecture,
--           0017_inspection_cache, 0018_cleanup, 0021_registry_columns, 0022_registry_cleanup
--
-- Final schema only — no temporary tables, no columns that get immediately dropped.

-- ═══════════════════════════════════════════════════════════════════════════════
-- Session tables
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    title TEXT,
    status TEXT NOT NULL DEFAULT 'active', -- active, paused, closed
    current_provider TEXT NOT NULL,
    provider_command TEXT NOT NULL,
    provider_args_json TEXT,
    working_directory TEXT NOT NULL,
    persona_dir TEXT,
    system_prompt TEXT,
    skills_json TEXT,
    provider_state_json TEXT, -- opaque state for resuming with same provider
    runtime_id TEXT,
    llm_provider_id TEXT,
    model_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_activity_at TEXT,
    closed_at TEXT
);

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

-- Session indexes
CREATE INDEX IF NOT EXISTS idx_session_messages_session_id ON session_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_session_messages_sequence ON session_messages(session_id, sequence_num);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_updated ON agent_sessions(updated_at);
CREATE INDEX IF NOT EXISTS idx_tool_results_session ON session_tool_results(session_id, tool_call_id);

-- ═══════════════════════════════════════════════════════════════════════════════
-- Agent runtimes (ACP)
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS agent_runtimes (
    id TEXT PRIMARY KEY,
    runtime_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT,
    command TEXT NOT NULL,
    args_json TEXT NOT NULL DEFAULT '[]',
    installation_method TEXT NOT NULL,
    is_bundled INTEGER NOT NULL DEFAULT 0,
    is_installed INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'not_installed',
    status_message TEXT,
    config_json TEXT,
    inspection_json TEXT, -- cached runtime capabilities
    inspected_at TEXT,
    registry_id TEXT, -- ACP registry agent ID (e.g., "gemini", "cursor")
    registry_version TEXT, -- version from ACP registry
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Runtime indexes
CREATE INDEX IF NOT EXISTS idx_agent_runtimes_default
    ON agent_runtimes(is_default) WHERE is_default = 1;
CREATE INDEX IF NOT EXISTS idx_agent_runtimes_status
    ON agent_runtimes(status);
CREATE INDEX IF NOT EXISTS idx_agent_runtimes_registry_id
    ON agent_runtimes(registry_id) WHERE registry_id IS NOT NULL;
