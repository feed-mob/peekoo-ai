-- @migrate: create
-- @id: 0015_agent_provider_configs
-- @sentinel: agent_providers

-- Agent provider configurations
CREATE TABLE IF NOT EXISTS agent_providers (
    id TEXT PRIMARY KEY,
    -- Provider identification
    provider_id TEXT NOT NULL UNIQUE, -- 'pi-acp', 'opencode', 'claude-code', 'codex', or custom slug
    display_name TEXT NOT NULL,
    description TEXT,
    is_bundled INTEGER NOT NULL DEFAULT 0, -- 1 for built-in, 0 for custom
    
    -- Installation configuration
    installation_method TEXT NOT NULL, -- 'bundled', 'npx', 'binary', 'custom_command'
    command TEXT, -- command to spawn (e.g., 'npx', '/path/to/agent')
    args_json TEXT, -- JSON array of arguments
    binary_path TEXT, -- for 'binary' or 'custom_command' method
    download_url TEXT, -- for downloading binaries
    checksum TEXT, -- sha256 of binary for verification
    
    -- Status
    is_installed INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0, -- 1 = default provider
    status TEXT NOT NULL DEFAULT 'not_installed', -- 'not_installed', 'installing', 'ready', 'error', 'needs_setup'
    status_message TEXT, -- human-readable status/error
    installed_at TEXT,
    updated_at TEXT NOT NULL,
    
    -- Provider-specific configuration (opaque JSON)
    config_json TEXT,
    
    -- Environment variables to pass to agent
    env_vars_json TEXT
);

-- Index for default provider lookup
CREATE INDEX IF NOT EXISTS idx_agent_providers_default 
    ON agent_providers(is_default) WHERE is_default = 1;

-- Index for ready providers
CREATE INDEX IF NOT EXISTS idx_agent_providers_status 
    ON agent_providers(status);

-- Provider installation tracking
CREATE TABLE IF NOT EXISTS agent_provider_installations (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    method TEXT NOT NULL, -- how it was installed
    version TEXT, -- installed version
    download_url TEXT,
    installed_at TEXT NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES agent_providers(id) ON DELETE CASCADE
);

-- Session-to-provider mapping (for tracking provider switches)
CREATE TABLE IF NOT EXISTS agent_session_providers (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    switched_at TEXT NOT NULL,
    reason TEXT, -- why the switch happened
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES agent_providers(id)
);

CREATE INDEX IF NOT EXISTS idx_session_providers_session 
    ON agent_session_providers(session_id);
