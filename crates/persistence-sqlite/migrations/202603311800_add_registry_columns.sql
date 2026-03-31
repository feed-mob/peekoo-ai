-- @migrate: alter
-- @id: 0021_add_registry_columns
-- @tolerates: "duplicate column name"

-- Add registry source tracking to agent_runtimes
-- This enables ACP registry integration with 40+ agents

-- Track where the agent came from: builtin (seeded), acp_registry (from CDN), custom (user-added)
ALTER TABLE agent_runtimes ADD COLUMN registry_source TEXT;

-- The registry ID from ACP registry (e.g., "gemini", "cursor", "goose")
ALTER TABLE agent_runtimes ADD COLUMN registry_id TEXT;

-- Version from the registry
ALTER TABLE agent_runtimes ADD COLUMN registry_version TEXT;

-- Additional metadata from registry: authors, license, website, icon_url, etc.
ALTER TABLE agent_runtimes ADD COLUMN registry_metadata TEXT;

-- Last time this agent was synced from registry
ALTER TABLE agent_runtimes ADD COLUMN last_registry_sync TEXT;

-- Index for efficient registry lookups
CREATE INDEX IF NOT EXISTS idx_agent_runtimes_registry_id 
    ON agent_runtimes(registry_id) WHERE registry_id IS NOT NULL;

-- Index for filtering by source
CREATE INDEX IF NOT EXISTS idx_agent_runtimes_registry_source 
    ON agent_runtimes(registry_source);
