import { describe, expect, test } from "bun:test";
import { getAgentWorkStatusBadge } from "./task-agent-work";

const mockT = (key: string) => key;

describe("getAgentWorkStatusBadge", () => {
  test("returns pending badge config", () => {
    expect(getAgentWorkStatusBadge("pending", mockT)).toEqual({
      label: "tasks.agentWork.pending",
      color: "#F59E0B",
      animated: false,
    });
  });

  test("returns executing badge config", () => {
    expect(getAgentWorkStatusBadge("executing", mockT)).toEqual({
      label: "tasks.agentWork.executing",
      color: "#3B82F6",
      animated: true,
    });
  });

  test("returns failed badge config", () => {
    expect(getAgentWorkStatusBadge("failed", mockT)).toEqual({
      label: "tasks.agentWork.failed",
      color: "#EF4444",
      animated: false,
    });
  });

  test("returns null for hidden statuses", () => {
    expect(getAgentWorkStatusBadge("claimed", mockT)).toBeNull();
    expect(getAgentWorkStatusBadge("completed", mockT)).toBeNull();
    expect(getAgentWorkStatusBadge(undefined, mockT)).toBeNull();
  });
});
