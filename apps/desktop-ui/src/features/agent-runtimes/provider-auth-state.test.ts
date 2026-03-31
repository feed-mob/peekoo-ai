import { describe, expect, test } from "bun:test";
import { getProviderAuthState, getProviderStatusText } from "./provider-auth-state";

describe("getProviderAuthState", () => {
  test("treats authRequired as the only login-required signal", () => {
    const state = getProviderAuthState({
      authRequired: false,
      authMethods: [{ id: "browser", name: "Browser Login" }],
    });

    expect(state.requiresAuth).toBe(false);
    expect(state.loginAvailable).toBe(true);
  });

  test("marks login required only when inspection says so", () => {
    const state = getProviderAuthState({
      authRequired: true,
      authMethods: [{ id: "browser", name: "Browser Login" }],
    });

    expect(state.requiresAuth).toBe(true);
    expect(state.loginAvailable).toBe(false);
  });
});

describe("getProviderStatusText", () => {
  test("shows login available instead of login required when auth is optional", () => {
    expect(
      getProviderStatusText("ready", {
        authRequired: false,
        authMethods: [{ id: "browser", name: "Browser Login" }],
      }),
    ).toBe("Login Available");
  });

  test("keeps login required wording when auth is required", () => {
    expect(
      getProviderStatusText("ready", {
        authRequired: true,
        authMethods: [{ id: "browser", name: "Browser Login" }],
      }),
    ).toBe("Login Required");
  });
});
