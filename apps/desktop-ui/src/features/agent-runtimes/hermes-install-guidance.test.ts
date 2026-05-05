import { describe, expect, test } from "bun:test";
import {
  HERMES_AVAILABLE_RUNTIME_ICON_URL,
  shouldShowHermesInstallGuidance,
} from "./hermes-install-guidance";

describe("shouldShowHermesInstallGuidance", () => {
  test("shows guidance when Hermes is not installed", () => {
    expect(shouldShowHermesInstallGuidance([])).toBe(true);
  });

  test("hides guidance when Hermes is already installed", () => {
    expect(
      shouldShowHermesInstallGuidance([
        { providerId: "hermes-agent", isInstalled: true },
      ]),
    ).toBe(false);
  });

  test("uses the Hermes logo for the available runtime card", () => {
    expect(HERMES_AVAILABLE_RUNTIME_ICON_URL).toBe(
      "https://hermes-agent.nousresearch.com/docs/img/logo.png",
    );
  });
});
