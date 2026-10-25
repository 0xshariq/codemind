import { describe, expect, it } from "vitest";
import { projectForUi } from "./ui-projection";

describe("UI result projection", () => {
  it("normalizes canonical identity fields into the browser DTO", () => {
    const result = projectForUi({ outcome: "success", completeness: "complete", freshness: "current", context: { repository_id: "repo", workspace_id: "workspace", source_snapshot_id: "snapshot", graph_version: "graph" }, data: { selected_entity_id: "symbol", impact_paths: [] } });
    expect(result.context).toEqual({ repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph" });
    expect(result.data.selected_entity_id).toBe("symbol");
  });
  it("rejects malformed context instead of inventing provenance", () => {
    expect(() => projectForUi({ outcome: "success", data: {}, context: { repositoryId: "repo" } })).toThrow(/context/i);
  });
});
