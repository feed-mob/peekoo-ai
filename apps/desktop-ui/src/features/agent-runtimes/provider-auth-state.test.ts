import { describe, expect, test } from "bun:test";
import { getProviderAuthState, getProviderLoginPresentation, getProviderStatusText } from "./provider-auth-state";

const mockT = ((key: string) => key) as import("i18next").TFunction;

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

  test("treats native login as available even without ACP auth methods", () => {
    const state = getProviderAuthState({
      authRequired: false,
      authMethods: [],
      nativeLoginCommand: "kimi login",
      preferredLoginMethod: "native",
    });

    expect(state.requiresAuth).toBe(false);
    expect(state.loginAvailable).toBe(true);
  });
});

describe("getProviderLoginPresentation", () => {
  test("prefers native login when backend marks it preferred", () => {
    const presentation = getProviderLoginPresentation({
      authRequired: true,
      authMethods: [{ id: "browser", name: "Browser Login" }],
      nativeLoginCommand: "kimi login",
      preferredLoginMethod: "native",
    });

    expect(presentation.primaryLoginMethod).toBe("native");
    expect(presentation.shouldShowNativeLogin).toBe(true);
    expect(presentation.shouldShowAcpLogin).toBe(false);
    expect(presentation.hasAcpFallback).toBe(true);
  });

  test("uses ACP login as primary when native is not preferred", () => {
    const presentation = getProviderLoginPresentation({
      authRequired: true,
      authMethods: [{ id: "browser", name: "Browser Login" }],
      nativeLoginCommand: "some login",
      preferredLoginMethod: "acp",
    });

    expect(presentation.primaryLoginMethod).toBe("acp");
    expect(presentation.shouldShowNativeLogin).toBe(false);
    expect(presentation.shouldShowAcpLogin).toBe(true);
    expect(presentation.hasAcpFallback).toBe(false);
  });
});

describe("getProviderStatusText", () => {
  test("shows login available instead of login required when auth is optional", () => {
    expect(
      getProviderStatusText("ready", {
        authRequired: false,
        authMethods: [{ id: "browser", name: "Browser Login" }],
        nativeLoginCommand: null,
        preferredLoginMethod: null,
      }, null, mockT),
    ).toBe("agentRuntimes.status.loginAvailable");
  });

  test("keeps login required wording when auth is required", () => {
    expect(
      getProviderStatusText("ready", {
        authRequired: true,
        authMethods: [{ id: "browser", name: "Browser Login" }],
        nativeLoginCommand: null,
        preferredLoginMethod: null,
      }, null, mockT),
    ).toBe("agentRuntimes.status.loginRequired");
  });
});
