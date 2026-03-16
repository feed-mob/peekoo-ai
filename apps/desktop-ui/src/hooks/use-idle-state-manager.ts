import { useState, useEffect, useRef, useCallback } from "react";

// Configuration for idle state management
const CONFIG = {
  IDLE_TIMEOUT_MIN: 120000,     // 最小空闲时间 2分钟
  IDLE_TIMEOUT_MAX: 180000,     // 最大空闲时间 3分钟
  RANDOM_STATE_DURATION_MIN: 15000,  // 随机状态最短 15秒
  RANDOM_STATE_DURATION_MAX: 90000,  // 随机状态最长 90秒
  ENABLE_RANDOM_STATES: true,   // 是否启用随机状态
};

// State weights for random selection
const STATE_WEIGHTS = {
  sleepy: 0.4,   // 40% 概率
  happy: 0.3,    // 30% 概率
  working: 0.2,  // 20% 概率
  thinking: 0.1, // 10% 概率
} as const;

type RandomState = keyof typeof STATE_WEIGHTS;

interface UseIdleStateManagerOptions {
  enabled?: boolean;
  onStateChange?: (state: string | null) => void;
  isUserInteracting?: boolean;
  hasActiveNotification?: boolean;
}

/**
 * Hook to manage random state transitions when sprite is idle
 * 
 * When the sprite has been in idle state for 2-3 minutes without user interaction,
 * it will randomly switch to sleepy/happy/working/thinking state for 15-90 seconds
 * before returning to idle.
 */
export function useIdleStateManager({
  enabled = CONFIG.ENABLE_RANDOM_STATES,
  onStateChange,
  isUserInteracting = false,
  hasActiveNotification = false,
}: UseIdleStateManagerOptions = {}) {
  const [randomState, setRandomState] = useState<string | null>(null);
  const idleTimerRef = useRef<number | null>(null);
  const stateTimerRef = useRef<number | null>(null);
  const lastInteractionRef = useRef<number>(Date.now());

  // Select a random state based on weights
  const selectRandomState = useCallback((): RandomState => {
    const random = Math.random();
    let cumulative = 0;

    for (const [state, weight] of Object.entries(STATE_WEIGHTS)) {
      cumulative += weight;
      if (random <= cumulative) {
        return state as RandomState;
      }
    }

    return "sleepy"; // Fallback
  }, []);

  // Get random duration within configured range
  const getRandomDuration = useCallback((min: number, max: number): number => {
    return Math.floor(Math.random() * (max - min + 1)) + min;
  }, []);

  // Clear all timers
  const clearTimers = useCallback(() => {
    if (idleTimerRef.current !== null) {
      window.clearTimeout(idleTimerRef.current);
      idleTimerRef.current = null;
    }
    if (stateTimerRef.current !== null) {
      window.clearTimeout(stateTimerRef.current);
      stateTimerRef.current = null;
    }
  }, []);

  // Activate a random state
  const activateRandomState = useCallback(() => {
    const state = selectRandomState();
    const duration = getRandomDuration(
      CONFIG.RANDOM_STATE_DURATION_MIN,
      CONFIG.RANDOM_STATE_DURATION_MAX
    );

    setRandomState(state);
    onStateChange?.(state);

    // Schedule return to idle
    stateTimerRef.current = window.setTimeout(() => {
      setRandomState(null);
      onStateChange?.(null);
      stateTimerRef.current = null;
      
      // Restart idle detection after returning to idle
      scheduleRandomState();
    }, duration);
  }, [selectRandomState, getRandomDuration, onStateChange]);

  // Schedule next random state transition
  const scheduleRandomState = useCallback(() => {
    if (!enabled) return;

    clearTimers();

    const timeout = getRandomDuration(
      CONFIG.IDLE_TIMEOUT_MIN,
      CONFIG.IDLE_TIMEOUT_MAX
    );

    idleTimerRef.current = window.setTimeout(() => {
      // Only activate if still idle (no interaction or notification)
      if (!isUserInteracting && !hasActiveNotification) {
        activateRandomState();
      } else {
        // Reschedule if conditions not met
        scheduleRandomState();
      }
    }, timeout);
  }, [enabled, isUserInteracting, hasActiveNotification, activateRandomState, getRandomDuration, clearTimers]);

  // Reset idle timer on user interaction
  const resetIdleTimer = useCallback(() => {
    lastInteractionRef.current = Date.now();
    
    // If in random state, exit it immediately
    if (randomState !== null) {
      setRandomState(null);
      onStateChange?.(null);
    }

    // Restart idle detection
    scheduleRandomState();
  }, [randomState, onStateChange, scheduleRandomState]);

  // Initialize and handle state changes
  useEffect(() => {
    if (!enabled) {
      clearTimers();
      return;
    }

    // Start idle detection
    scheduleRandomState();

    return () => {
      clearTimers();
    };
  }, [enabled, scheduleRandomState, clearTimers]);

  // Handle user interaction changes
  useEffect(() => {
    if (isUserInteracting || hasActiveNotification) {
      resetIdleTimer();
    }
  }, [isUserInteracting, hasActiveNotification, resetIdleTimer]);

  return {
    randomState,
    resetIdleTimer,
    isActive: randomState !== null,
  };
}
