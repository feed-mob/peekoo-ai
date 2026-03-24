-- Migration 0009: Agent Task Assignment
-- Adds agent work tracking columns to tasks and creates agent_registry table

-- Update existing assignee values from 'agent' to 'peekoo-agent'
UPDATE tasks SET assignee = 'peekoo-agent' WHERE assignee = 'agent';

-- Add agent work tracking columns to tasks
ALTER TABLE tasks ADD COLUMN agent_work_status TEXT DEFAULT 'pending';
ALTER TABLE tasks ADD COLUMN agent_work_session_id TEXT;
ALTER TABLE tasks ADD COLUMN agent_work_attempt_count INTEGER DEFAULT 0;
ALTER TABLE tasks ADD COLUMN agent_work_started_at TEXT;
ALTER TABLE tasks ADD COLUMN agent_work_completed_at TEXT;

-- Set agent_work_status to 'pending' for existing agent-assigned tasks
UPDATE tasks SET agent_work_status = 'pending' WHERE assignee != 'user' AND status != 'done';
-- User-assigned tasks don't need agent work tracking
UPDATE tasks SET agent_work_status = NULL WHERE assignee = 'user';

-- Create agent_registry table
CREATE TABLE IF NOT EXISTS agent_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    config_json TEXT NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

-- Insert default peekoo-agent entry
INSERT OR IGNORE INTO agent_registry (id, name, command, capabilities_json, created_at)
VALUES (
    'peekoo-agent',
    'Peekoo Agent',
    'peekoo-agent-acp',
    '["task_planning", "task_execution", "question_asking"]',
    datetime('now')
);

-- Index for scheduler queries
CREATE INDEX IF NOT EXISTS idx_tasks_agent_execution 
ON tasks(assignee, scheduled_start_at, status, agent_work_status) 
WHERE assignee != 'user';
