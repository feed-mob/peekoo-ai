import { describe, expect, test } from "bun:test";
import { shouldShowHermesInstallGuidance } from "./hermes-install-guidance";

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
});
