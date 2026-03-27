# Pomodoro Daily Counter Reset

**Date**: 2026-03-26  
**Type**: Feature  
**Scope**: Pomodoro System

## Problem

The pomodoro badge counters (`completed_focus` and `completed_breaks`) accumulated indefinitely without resetting, showing historical totals instead of daily progress. This made the badge less useful for tracking today's productivity.

## Solution

Implemented automatic daily reset of pomodoro counters:

1. **Database Migration** (`0013_pomodoro_daily_reset.sql`):
   - Added `last_reset_date TEXT` column to `pomodoro_state` table
   - Initialized with current date for existing data

2. **Domain Model** (`peekoo-pomodoro-domain`):
   - Added `last_reset_date: Option<String>` field to `PomodoroStatus`
   - Updated constructor and tests

3. **App Service** (`peekoo-pomodoro-app`):
   - Updated `load_status()` to read `last_reset_date` from database
   - Updated `save_status()` to persist `last_reset_date`
   - Updated `ensure_seed_row()` to initialize with current date
   - Added daily reset logic in `reconcile_runtime_state()`:
     - Checks if `last_reset_date` differs from today
     - Resets `completed_focus` and `completed_breaks` to 0
     - Updates `last_reset_date` to today

4. **Migration Registration** (`peekoo-agent-app/settings/store.rs`):
   - Added `MIGRATION_0013_POMODORO_DAILY_RESET` import
   - Registered migration with ID `0013_pomo_daily_reset_v1`

## Behavior

- On app startup, the system checks if the date has changed
- If a new day is detected, counters reset to 0 automatically
- Badge now shows today's completed sessions only
- Reset happens once per day on first app launch

## Files Modified

- `crates/persistence-sqlite/migrations/0013_pomodoro_daily_reset.sql` (new)
- `crates/persistence-sqlite/src/lib.rs`
- `crates/peekoo-pomodoro-domain/src/lib.rs`
- `crates/peekoo-pomodoro-app/src/lib.rs`
- `crates/peekoo-agent-app/src/settings/store.rs`

## Testing

- Domain model tests pass
- Code compiles successfully across all affected crates
- Migration will be applied automatically on next app launch
