# Pomodoro History Date Filter

**Date**: 2026-03-26  
**Type**: Feature  
**Scope**: Pomodoro System

## Problem

The pomodoro history panel only showed the most recent 6 sessions, making it difficult to review past productivity patterns or find specific sessions from earlier dates.

## Solution

Implemented date-based filtering for pomodoro history:

1. **Backend API** (`peekoo-pomodoro-app`):
   - Added `history_by_date_range(start_date, end_date, limit)` method
   - Queries database using SQL `date(ended_at) BETWEEN ?1 AND ?2`
   - Returns up to 50 sessions within the date range

2. **Application Layer** (`peekoo-agent-app`):
   - Exposed `pomodoro_history_by_date_range()` method
   - Passes date range parameters to pomodoro service

3. **Tauri Commands** (`desktop-tauri`):
   - Added `pomodoro_history_by_date_range` command
   - Registered in invoke_handler

4. **Frontend API Client** (`tool-client.ts`):
   - Added `getPomodoroHistoryByDateRange()` function
   - Accepts startDate, endDate, and limit parameters

5. **UI Component** (`PomodoroPanel.tsx`):
   - Added date filter dropdown with 5 options:
     - Recent 6 (default, shows last 6 sessions)
     - Today
     - Yesterday
     - Last 7 Days
     - Last 30 Days
   - Implemented `getDateRange()` helper to calculate date ranges
   - Updated `fetchStatus()` to use filtered query when date range selected
   - Increased history limit from 6 to 50 for date-filtered queries

## Features

- **Dropdown Filter**: Clean, compact select menu in history section header
- **Smart Date Calculation**: Automatically calculates date ranges based on selection
- **Backward Compatible**: "Recent 6" option maintains original behavior
- **Responsive**: Updates history immediately when filter changes
- **Styled Consistently**: Matches existing UI design language

## Date Range Logic

```typescript
- Today: Current date only
- Yesterday: Previous day only
- Last 7 Days: Today minus 6 days to today (inclusive)
- Last 30 Days: Today minus 29 days to today (inclusive)
- Recent 6: No date filter, returns 6 most recent sessions
```

## Files Modified

- `crates/peekoo-pomodoro-app/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/features/pomodoro/tool-client.ts`
- `apps/desktop-ui/src/features/pomodoro/PomodoroPanel.tsx`

## Testing

- Backend code compiles successfully
- Date range calculation tested for edge cases
- UI dropdown integrates seamlessly with existing design
