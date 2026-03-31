-- @migrate: alter
-- @id: 0022_registry_source_of_truth

-- Remove uninstalled non-bundled rows (hardcoded seeds that were never installed)
DELETE FROM agent_runtimes WHERE is_installed = 0 AND is_bundled = 0;

-- Ensure any existing installed opencode row has registry_id set
UPDATE agent_runtimes
SET registry_id = 'opencode'
WHERE runtime_type = 'opencode' AND (registry_id IS NULL OR registry_id = '');

-- Drop indexes that reference columns we're about to drop
DROP INDEX IF EXISTS idx_agent_runtimes_registry_source;

-- Drop columns that are written but never read back
ALTER TABLE agent_runtimes DROP COLUMN is_enabled;
ALTER TABLE agent_runtimes DROP COLUMN install_hint;
ALTER TABLE agent_runtimes DROP COLUMN registry_source;
ALTER TABLE agent_runtimes DROP COLUMN registry_metadata;
ALTER TABLE agent_runtimes DROP COLUMN last_registry_sync;
