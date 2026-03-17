# Changelog: Idle State Random Transitions

**Date**: 2026-03-14 18:00  
**Type**: Feature  
**Component**: Desktop UI - Sprite Animation System

## Summary

Implemented a random state transition system for the desktop sprite to make it more lively and engaging. When the sprite has been idle for 2-3 minutes without user interaction, it will randomly switch to different states (sleepy, happy, working, thinking) for 15-90 seconds before returning to idle.

## Changes

### New Files

1. **`apps/desktop-ui/src/hooks/use-idle-state-manager.ts`**
   - New hook for managing idle state detection and random transitions
   - Configurable timing parameters (idle timeout: 2-3 min, state duration: 15-90 sec)
   - Weighted random state selection:
     - Sleepy: 40%
     - Happy: 30%
     - Working: 20%
     - Thinking: 10%
   - Respects user interaction and notification priorities
   - Automatically resets on user interaction

### Modified Files

1. **`apps/desktop-ui/src/views/SpriteView.tsx`**
   - Integrated `useIdleStateManager` hook
   - Added `randomState` to sprite state priority system
   - State priority: moodOverride (reactions/reminders) > randomState > default spriteState
   - Added `resetIdleTimer()` calls to user interaction handlers:
     - `handleMouseDown` (drag start)
     - `handleContextMenu` (right-click menu)
     - `handleTogglePanel` (panel toggle)
   - Configured idle manager to pause during:
     - Menu open
     - Dragging
     - Active notifications/reminders
     - Bubble visible

## Technical Details

### State Priority System

```typescript
effectiveSpriteState = moodOverride 
  ? { ...spriteState, mood: moodOverride }      // Highest priority
  : randomState
  ? { ...spriteState, mood: randomState }       // Medium priority
  : spriteState;                                 // Default
```

### Idle Detection Logic

- Tracks user interaction timestamps
- Uses two timers:
  - `idleTimer`: Waits 2-3 minutes before triggering random state
  - `stateTimer`: Controls random state duration (15-90 seconds)
- Automatically clears and restarts timers on interaction
- Pauses when user is actively interacting or notifications are present

### Configuration

```typescript
const CONFIG = {
  IDLE_TIMEOUT_MIN: 120000,     // 2 minutes
  IDLE_TIMEOUT_MAX: 180000,     // 3 minutes
  RANDOM_STATE_DURATION_MIN: 15000,  // 15 seconds
  RANDOM_STATE_DURATION_MAX: 90000,  // 90 seconds
  ENABLE_RANDOM_STATES: true,
};
```

## User Experience

- Sprite appears more alive and less static during long idle periods
- Random transitions feel natural and non-intrusive
- User interactions immediately override random states
- System notifications and reminders take priority over random states
- Smooth transitions between states

## Testing

- Type checking passed: `npx tsc --noEmit`
- Manual testing recommended:
  1. Leave sprite idle for 2-3 minutes
  2. Observe random state transitions
  3. Verify user interaction resets idle timer
  4. Confirm notifications override random states

## Future Enhancements

Potential improvements for future iterations:
- Make timing parameters user-configurable via settings
- Add more state variety or context-aware states
- Track state transition history for analytics
- Add smooth animation transitions between states
- Consider time-of-day based state preferences (e.g., more sleepy states in evening)

## Related Files

- `apps/desktop-ui/src/components/sprite/Sprite.tsx` - Sprite component with mood mapping
- `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx` - Animation row definitions
- `apps/desktop-ui/src/types/sprite.ts` - Type definitions for sprite states
