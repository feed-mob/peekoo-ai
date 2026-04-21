import type { RuntimeAuthenticationResult } from "@/types/agent-runtime";

export interface RuntimeAuthFeedbackAlert {
  success: boolean;
  message: string;
}

export interface RuntimeAuthFeedbackState {
  awaitingManualRefresh: boolean;
  alert: RuntimeAuthFeedbackAlert | null;
}

export function getAuthFeedbackAfterRuntimeAuthentication(
  result: RuntimeAuthenticationResult,
): RuntimeAuthFeedbackState {
  if (result.status === "authenticated") {
    return {
      awaitingManualRefresh: false,
      alert: {
        success: true,
        message: result.message,
      },
    };
  }

  return {
    awaitingManualRefresh: true,
    alert: null,
  };
}

export function getAuthFeedbackAfterRefresh(
  awaitingManualRefresh: boolean,
  authRequired: boolean,
  completedMessage: string,
): RuntimeAuthFeedbackState {
  if (!awaitingManualRefresh) {
    return {
      awaitingManualRefresh: false,
      alert: null,
    };
  }

  if (authRequired) {
    return {
      awaitingManualRefresh: true,
      alert: null,
    };
  }

  return {
    awaitingManualRefresh: false,
    alert: {
      success: true,
      message: completedMessage,
    },
  };
}
