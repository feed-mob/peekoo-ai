-- Migration: Add created_at column to tasks table
-- This supports proper sorting by creation time

ALTER TABLE tasks ADD COLUMN created_at TEXT DEFAULT (datetime('now'));