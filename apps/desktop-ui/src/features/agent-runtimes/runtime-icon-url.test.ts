import { describe, expect, test } from "bun:test";
import { getRuntimeIconUrl } from "./runtime-icon-url";

describe("getRuntimeIconUrl", () => {
  test("uses the Hermes docs logo for Hermes Agent", () => {
    expect(getRuntimeIconUrl("hermes-agent")).toBe(
      "https://hermes-agent.nousresearch.com/docs/img/logo.png",
    );
  });

  test("uses ACP registry CDN icons for registry runtimes", () => {
    expect(getRuntimeIconUrl("opencode")).toBe(
      "https://cdn.agentclientprotocol.com/registry/v1/latest/opencode.svg",
    );
  });
});
