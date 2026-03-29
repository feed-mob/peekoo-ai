-- @migrate: alter
-- @id: 0018_cleanup_runtime_tables
-- @tolerates: "no such table"

-- Drop obsolete runtime_llm_providers table
-- This table is no longer needed as providers are discovered via ACP protocol
DROP TABLE IF EXISTS runtime_llm_providers;

-- Drop obsolete runtime_models table
-- This table is no longer needed as models are discovered via ACP protocol
DROP TABLE IF EXISTS runtime_models;
