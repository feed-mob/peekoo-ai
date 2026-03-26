-- Migration 0014: Add notes column to tasks table if missing
-- This handles databases created before notes was added to the initial schema

ALTER TABLE tasks ADD COLUMN notes TEXT;
