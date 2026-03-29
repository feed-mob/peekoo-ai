-- @migrate: alter
-- @id: 0017_runtime_inspection_cache
-- @tolerates: "duplicate column name"

ALTER TABLE agent_runtimes ADD COLUMN inspection_json TEXT;
ALTER TABLE agent_runtimes ADD COLUMN inspected_at TEXT;
