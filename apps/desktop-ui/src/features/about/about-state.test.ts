import { describe, expect, test } from "bun:test";
import { loadAboutSnapshot } from "./about-state";

describe("loadAboutSnapshot", () => {
  test("returns current version when no update is available", async () => {
    const result = await loadAboutSnapshot({
      getName: async () => "Peekoo",
      getVersion: async () => "0.1.2",
      check: async () => null,
    });

    expect(result.snapshot).toEqual({
      appName: "Peekoo",
      currentVersion: "0.1.2",
      availableVersion: null,
      releaseDate: null,
      releaseNotes: null,
      isUpdateAvailable: false,
    });
    expect(result.update).toBeNull();
  });

  test("returns new version metadata when an update is available", async () => {
    const fakeUpdate = {
      currentVersion: "0.1.2",
      version: "0.1.3",
      date: "2026-03-18T12:00:00Z",
      body: "Bug fixes and improvements",
    };

    const result = await loadAboutSnapshot({
      getName: async () => "Peekoo",
      getVersion: async () => "0.1.2",
      check: async () => fakeUpdate,
    });

    expect(result.snapshot).toEqual({
      appName: "Peekoo",
      currentVersion: "0.1.2",
      availableVersion: "0.1.3",
      releaseDate: "2026-03-18T12:00:00Z",
      releaseNotes: "Bug fixes and improvements",
      isUpdateAvailable: true,
    });
    expect(result.update).toEqual(fakeUpdate);
  });
});
