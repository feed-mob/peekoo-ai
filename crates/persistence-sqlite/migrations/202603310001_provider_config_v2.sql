-- @migrate: alter
-- @id: 0015_provider_config_v2
-- @tolerates: "no such column", "duplicate column name"
--
-- Consolidated migration for provider config consolidation.
-- Replaces: 0019_consolidate_provider_config
--
-- Move active provider tracking to agent_runtimes.is_default.
-- Clean up agent_settings to remove redundant columns.

-- Step 1: Promote the runtime matching active_provider_id (if any) to is_default.
UPDATE agent_runtimes SET is_default = 0;

UPDATE agent_runtimes
SET is_default = 1
WHERE runtime_type = (
    SELECT active_provider_id FROM agent_settings WHERE id = 1
)
AND EXISTS (
    SELECT 1 FROM agent_settings WHERE id = 1
);

-- Step 2: Fallback — if no runtime ended up as default, set opencode.
UPDATE agent_runtimes
SET is_default = 1
WHERE runtime_type = 'opencode'
AND NOT EXISTS (SELECT 1 FROM agent_runtimes WHERE is_default = 1);

-- Step 3: Bump version so the agent service recreates on next prompt.
UPDATE agent_settings
SET version = version + 1
WHERE id = 1;

-- Step 4: Recreate agent_settings without active_provider_id and active_model_id.
CREATE TABLE IF NOT EXISTS agent_settings_new (
    id INTEGER PRIMARY KEY,
    system_prompt TEXT,
    max_tool_iterations INTEGER NOT NULL DEFAULT 50,
    version INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO agent_settings_new (id, system_prompt, max_tool_iterations, version, updated_at)
SELECT id, system_prompt, max_tool_iterations, version, updated_at
FROM agent_settings;

DROP TABLE agent_settings;

ALTER TABLE agent_settings_new RENAME TO agent_settings;
