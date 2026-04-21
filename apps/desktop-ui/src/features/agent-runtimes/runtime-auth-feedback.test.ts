import { describe, expect, test } from "bun:test";
import {
  getAuthFeedbackAfterRefresh,
  getAuthFeedbackAfterRuntimeAuthentication,
} from "./runtime-auth-feedback";

describe("getAuthFeedbackAfterRuntimeAuthentication", () => {
  test("does not report terminal login started as a success alert", () => {
    expect(
      getAuthFeedbackAfterRuntimeAuthentication({
        status: "terminal_login_started",
        message: "Terminal login started.",
      }),
    ).toEqual({
      awaitingManualRefresh: true,
      alert: null,
    });
  });

  test("keeps immediate authenticated results as success alerts", () => {
    expect(
      getAuthFeedbackAfterRuntimeAuthentication({
        status: "authenticated",
        message: "Authenticated.",
      }),
    ).toEqual({
      awaitingManualRefresh: false,
      alert: {
        success: true,
        message: "Authenticated.",
      },
    });
  });
});

describe("getAuthFeedbackAfterRefresh", () => {
  test("keeps waiting without a stale success alert when login is still required", () => {
    expect(getAuthFeedbackAfterRefresh(true, true, "Login successful.")).toEqual({
      awaitingManualRefresh: true,
      alert: null,
    });
  });

  test("reports success after manual refresh confirms login completed", () => {
    expect(getAuthFeedbackAfterRefresh(true, false, "Login successful.")).toEqual({
      awaitingManualRefresh: false,
      alert: {
        success: true,
        message: "Login successful.",
      },
    });
  });

  test("leaves unrelated refreshes unchanged", () => {
    expect(getAuthFeedbackAfterRefresh(false, false, "Login successful.")).toEqual({
      awaitingManualRefresh: false,
      alert: null,
    });
  });
});
