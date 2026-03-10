# Countdown Persistence

How health reminder timers survive app restarts.

---

## Problem

The `Scheduler` stores `next_fire_at` as `std::time::Instant` (monotonic clock) in memory. On app restart, all timer state is lost and countdowns reset to their full configured interval.

## Solution

The plugin persists **wall-clock "fire at" timestamps** to SQLite via the existing `peekoo_state_set` host function. On init, it reads the stored timestamps and passes a computed `delay_secs` to the scheduler so the first fire resumes from where the timer left off.

---

## Scheduler API: `delay_secs`

`Scheduler::set()` signature:

```rust
pub fn set(
    &self,
    owner: &str,
    key: &str,
    interval_secs: u64,
    repeat: bool,
    delay_secs: Option<u64>,
) -> Result<(), SchedulerError>
```

| `delay_secs` | First fire at | Subsequent fires |
|--------------|---------------|------------------|
| `None` | `now + interval_secs` | `now + interval_secs` |
| `Some(n)` | `now + n` | `now + interval_secs` |
| `Some(0)` | immediately | `now + interval_secs` |

The host function `peekoo_schedule_set` accepts an optional `delay_secs` field in its JSON input:

```json
{
  "key": "water",
  "interval_secs": 2700,
  "repeat": true,
  "delay_secs": 1200
}
```

If `delay_secs` is absent or null, the full interval is used (backward compatible).

---

## Plugin State Keys

Stored per reminder type via `peekoo_state_set`:

| Key | Value | Written when |
|-----|-------|-------------|
| `timer_fire_at:water` | epoch seconds (u64) | Timer set, timer fires, dismiss |
| `timer_fire_at:eye_rest` | epoch seconds (u64) | Timer set, timer fires, dismiss |
| `timer_fire_at:standup` | epoch seconds (u64) | Timer set, timer fires, dismiss |
| `timer_interval:water` | seconds (u64) | Timer set (for stale detection) |
| `timer_interval:eye_rest` | seconds (u64) | Timer set (for stale detection) |
| `timer_interval:standup` | seconds (u64) | Timer set (for stale detection) |

---

## Persistence Flow

### On timer set (`schedule_set_with_delay`)

```
fire_at = now_epoch + delay_secs.unwrap_or(interval_secs)
state_set("timer_fire_at:<key>", fire_at)
state_set("timer_interval:<key>", interval_secs)
```

### On timer fire (`handle_schedule_fired`)

```
schedule = schedule_get(key)  // get current interval from live scheduler
state_set("timer_fire_at:<key>", now_epoch + schedule.interval_secs)
```

### On app restart (`sync_schedules` -> `compute_remaining_delay`)

```
(fire_at, stored_interval) = load from state store
if stored_interval != current_interval:
    return None  // interval changed, start fresh
if fire_at <= now:
    if fire_if_overdue:
        return Some(0)  // fire immediately
    else:
        // Skip missed reminder, compute position in next cycle
        overdue = now - fire_at
        into_next_cycle = overdue % interval_secs
        remaining = interval_secs - into_next_cycle
        return Some(remaining)
else:
    return Some(fire_at - now)  // resume with remaining time
```

Health reminders use `fire_if_overdue: false` -- missed reminders are skipped
and the delay is set to the remaining time in the next cycle. For example, if
a 45-min timer was overdue by 2 min, the delay is 43 min. Other plugins can
pass `fire_if_overdue: true` to fire immediately on restart instead.

---

## What Is NOT Persisted

| Data | Why |
|------|-----|
| Scheduler `ScheduleEntry` structs | Uses `Instant` (monotonic, non-serializable) |
| Peek badge items | Rebuilt from timer state on init |
| Active schedule list | Reconstructed from config + timestamps on init |

The scheduler remains purely in-memory. Only the plugin layer persists enough information to reconstruct timers.

---

## Key Functions

| Function | File | Purpose |
|----------|------|---------|
| `Scheduler::set()` | `crates/peekoo-scheduler/src/scheduler.rs` | Accepts `delay_secs` for initial fire offset |
| `host_schedule_set` | `crates/peekoo-plugin-host/src/host_functions.rs` | Passes `delay_secs` from JSON to scheduler |
| `schedule_set_with_delay()` | `plugins/health-reminders/src/lib.rs` | Sets timer + persists timestamps |
| `save_timer_started_at()` | `plugins/health-reminders/src/lib.rs` | Writes fire_at + interval to state store |
| `load_timer_fire_at()` | `plugins/health-reminders/src/lib.rs` | Reads fire_at + interval from state store |
| `compute_remaining_delay()` | `plugins/health-reminders/src/lib.rs` | Computes delay from stored vs current time; `fire_if_overdue` controls whether missed timers fire immediately or skip to next cycle |
| `current_epoch_secs()` | `plugins/health-reminders/src/lib.rs` | `SystemTime::now()` as epoch u64 |
