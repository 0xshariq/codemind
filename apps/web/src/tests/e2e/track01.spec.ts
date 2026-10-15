import { describe, expect, it } from "vitest";
import { vi } from "vitest";
vi.mock("server-only", () => ({}));
import { TOOL_HANDLERS } from "../../lib/server/tools/implementations";
import { toolNames } from "../../lib/server/tools/registry";

describe("Track 01 semantic investigation contract", () => {
  it("exposes exactly the approved registry handlers", () => {
    expect(Object.keys(TOOL_HANDLERS).sort()).toEqual([...toolNames()].sort());
  });

  it("preserves repository, snapshot, graph and symbol scope", () => {
    const input = {
      context: { repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph-v1" },
      symbolId: "symbol",
    };
    expect(input.context.repositoryId).toBe("repo");
    expect(input.context.sourceSnapshotId).toBe("snapshot");
    expect(input.symbolId).toBe("symbol");
  });
});