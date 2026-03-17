# Fix: Session Index Lock Timeout Issue

**Date**: 2026-03-13 16:00  
**Type**: Bug Fix  
**Component**: pi_agent/session_index, pi_agent/session

## Problem

Users experienced "拒绝访问 (os error 5)" errors when chatting with the desktop pet. The error occurred due to file lock contention on session files.

### Root Cause

1. The `SessionIndex::with_lock()` method had a 5-second timeout for acquiring the exclusive file lock
2. During streaming chat responses, multiple session saves could occur in quick succession
3. Each save operation calls `enqueue_session_index_snapshot_update()`, which attempts to acquire the lock
4. **More critically**: The session file append operation in `session.rs` had no retry mechanism when opening the file
5. If the file was locked by another operation, it would immediately fail with "os error 5"

## Solution

### Changes Made

**File 1**: `pi_agent/src/session_index.rs`

1. **Increased lock timeout from 5 to 30 seconds**
   - Changed `Duration::from_secs(5)` to `Duration::from_secs(30)` in `with_lock()`
   - This gives more time for concurrent operations to complete

2. **Improved lock acquisition logging**
   - Added retry attempt counter
   - Log debug message when lock is acquired after retries
   - Shows elapsed time for lock acquisition

3. **Enhanced error messages**
   - Error now includes number of attempts and timeout duration
   - Provides hint about possible causes (concurrent access or stale lock)

**File 2**: `pi_agent/src/session.rs`

1. **Added retry mechanism for file open operations (line ~1717)**
   - Retry up to 10 times with exponential backoff (50ms * attempt)
   - Log debug messages on retry attempts
   - Only fail after all retries exhausted

2. **Added retry mechanism for temp file persist operations (line ~1629)**
   - Retry up to 10 times with exponential backoff (50ms * attempt)
   - Handle `PersistError` which returns the file on failure
   - Log debug messages on retry attempts
   - Critical for Windows where file renames can fail due to locks

### Code Changes

**session_index.rs**:
```rust
// Before
let _lock = lock_file_guard(&lock_file, Duration::from_secs(5))?;

// After
let _lock = lock_file_guard(&lock_file, Duration::from_secs(30))?;
```

**session.rs**:
```rust
// Before
let mut file = std::fs::OpenOptions::new()
    .append(true)
    .open(&path_for_thread)
    .map_err(|e| crate::Error::Io(Box::new(e)))?;

// After
let mut file = None;
let max_attempts = 10;
for attempt in 1..=max_attempts {
    match std::fs::OpenOptions::new()
        .append(true)
        .open(&path_for_thread)
    {
        Ok(f) => {
            if attempt > 1 {
                tracing::debug!(
                    attempt = attempt,
                    "Opened session file after retries"
                );
            }
            file = Some(f);
            break;
        }
        Err(e) if attempt < max_attempts => {
            tracing::debug!(
                attempt = attempt,
                error = %e,
                "Failed to open session file, retrying"
            );
            std::thread::sleep(std::time::Duration::from_millis(50 * attempt as u64));
        }
        Err(e) => {
            return Err(crate::Error::Io(Box::new(e)));
        }
    }
}
let mut file = file.ok_or_else(|| {
    crate::Error::session("Failed to open session file after retries")
})?;
```

**session.rs** (temp file persist retry):
```rust
// Before
temp_file
    .persist(&path_for_thread)
    .map_err(|e| crate::Error::Io(Box::new(e.error)))?;

// After
let mut temp_file_opt = Some(temp_file);
let max_attempts = 10;
for attempt in 1..=max_attempts {
    let temp = temp_file_opt.take().ok_or_else(|| {
        crate::Error::session("Temp file consumed unexpectedly")
    })?;
    match temp.persist(&path_for_thread) {
        Ok(_) => {
            if attempt > 1 {
                tracing::debug!(
                    attempt = attempt,
                    "Persisted temp file after retries"
                );
            }
            break;
        }
        Err(e) if attempt < max_attempts => {
            tracing::debug!(
                attempt = attempt,
                error = %e.error,
                "Failed to persist temp file, retrying"
            );
            temp_file_opt = Some(e.file);
            std::thread::sleep(std::time::Duration::from_millis(50 * attempt as u64));
        }
        Err(e) => {
            return Err(crate::Error::Io(Box::new(e.error)));
        }
    }
}
```

## Impact

- Users should no longer experience "拒绝访问" errors during normal chat operations
- Better visibility into lock contention through debug logs
- More informative error messages if timeout still occurs
- Graceful handling of transient file lock contention

## Testing

- Manual testing: Chat with desktop pet multiple times in quick succession
- Monitor logs for retry messages with `RUST_LOG=debug`
- Verify no errors occur during streaming responses

## Future Improvements

Consider these optimizations if lock contention remains an issue:

1. **Batch index updates**: Queue multiple updates and process them together
2. **Async lock acquisition**: Use tokio's async file locking
3. **Lock-free index**: Use a different data structure that doesn't require exclusive locks
4. **Separate read/write locks**: Allow concurrent reads while serializing writes
