import { describe, expect, test } from "bun:test";
import { getAgentWorkStatusBadge } from "./task-agent-work";

describe("getAgentWorkStatusBadge", () => {
  test("returns pending badge config", () => {
    expect(getAgentWorkStatusBadge("pending")).toEqual({
      label: "Pending",
      color: "#F59E0B",
      animated: false,
    });
  });

  test("returns executing badge config", () => {
    expect(getAgentWorkStatusBadge("executing")).toEqual({
      label: "Executing",
      color: "#3B82F6",
      animated: true,
    });
  });

  test("returns failed badge config", () => {
    expect(getAgentWorkStatusBadge("failed")).toEqual({
      label: "Failed",
      color: "#EF4444",
      animated: false,
    });
  });

  test("returns null for hidden statuses", () => {
    expect(getAgentWorkStatusBadge("claimed")).toBeNull();
    expect(getAgentWorkStatusBadge("completed")).toBeNull();
    expect(getAgentWorkStatusBadge(undefined)).toBeNull();
  });
});
