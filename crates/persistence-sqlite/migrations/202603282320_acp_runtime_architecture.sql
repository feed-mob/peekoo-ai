-- @migrate: alter
-- @id: 0016_acp_runtime_architecture
-- @tolerates: "duplicate column name"

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
    is_enabled INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'not_installed',
    status_message TEXT,
    install_hint TEXT,
    config_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_runtimes_default
    ON agent_runtimes(is_default) WHERE is_default = 1;

CREATE INDEX IF NOT EXISTS idx_agent_runtimes_status
    ON agent_runtimes(status);

CREATE TABLE IF NOT EXISTS runtime_llm_providers (
    id TEXT PRIMARY KEY,
    runtime_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    display_name TEXT,
    api_type TEXT NOT NULL,
    base_url TEXT,
    config_json TEXT NOT NULL DEFAULT '{}',
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (runtime_id) REFERENCES agent_runtimes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_runtime_llm_providers_runtime
    ON runtime_llm_providers(runtime_id);

CREATE INDEX IF NOT EXISTS idx_runtime_llm_providers_default
    ON runtime_llm_providers(runtime_id, is_default) WHERE is_default = 1;

CREATE TABLE IF NOT EXISTS runtime_models (
    id TEXT PRIMARY KEY,
    runtime_id TEXT NOT NULL,
    provider_id TEXT,
    model_id TEXT NOT NULL,
    display_name TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (runtime_id) REFERENCES agent_runtimes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_runtime_models_runtime
    ON runtime_models(runtime_id);

CREATE INDEX IF NOT EXISTS idx_runtime_models_default
    ON runtime_models(runtime_id, is_default) WHERE is_default = 1;

ALTER TABLE agent_sessions ADD COLUMN runtime_id TEXT;
ALTER TABLE agent_sessions ADD COLUMN llm_provider_id TEXT;
ALTER TABLE agent_sessions ADD COLUMN model_id TEXT;

INSERT OR IGNORE INTO agent_runtimes (
    id,
    runtime_type,
    display_name,
    description,
    command,
    args_json,
    installation_method,
    is_bundled,
    is_installed,
    is_default,
    status,
    install_hint,
    config_json,
    created_at,
    updated_at
)
SELECT
    id,
    provider_id,
    display_name,
    description,
    COALESCE(command, provider_id),
    COALESCE(args_json, '[]'),
    installation_method,
    is_bundled,
    is_installed,
    is_default,
    status,
    NULL,
    COALESCE(config_json, '{}'),
    COALESCE(installed_at, updated_at, CURRENT_TIMESTAMP),
    updated_at
FROM agent_providers;

UPDATE agent_sessions
SET runtime_id = COALESCE(runtime_id, current_provider)
WHERE runtime_id IS NULL;
