import { describe, expect, test } from "bun:test";
import config from "./vite.config";

function resolveConfig() {
  return typeof config === "function"
    ? config({
        command: "serve",
        mode: "test",
        isSsrBuild: false,
        isPreview: false,
      })
    : config;
}

describe("vite config", () => {
  test("dedupes react packages", () => {
    const resolved = resolveConfig();

    expect(resolved.resolve?.dedupe).toEqual(
      expect.arrayContaining(["react", "react-dom"]),
    );
  });
});
