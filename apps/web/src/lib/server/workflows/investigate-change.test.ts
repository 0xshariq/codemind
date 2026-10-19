import { describe, expect, it, vi } from "vitest";

vi.mock("server-only", () => ({}));

import { initialEvidenceToolCalls } from "./investigate-change";
import { APPROVED_TOOL_NAMES } from "../tools/catalog";
import { parseToolInput } from "../tools/schemas";

describe("initial investigation evidence tools", () => {
  it("runs every approved semantic tool on the first graph-backed turn", () => {
    const calls = initialEvidenceToolCalls("What changes when I add a project?", "entity-1", true);
    const context = { workspaceId: "workspace", repositoryId: "repo", sourceSnapshotId: "snapshot", graphVersion: "graph" };

    expect(calls.filter((call) => call.name !== "search_code").map((call) => call.name)).toEqual(
      APPROVED_TOOL_NAMES.filter((name) => name !== "search_code"),
    );
    for (const call of calls) expect(() => parseToolInput(call.name, { ...call.input, context })).not.toThrow();
    expect(calls.filter((call) => call.name === "search_code").map((call) => call.input.query)).toEqual(["add", "project"]);
    expect(calls.filter((call) => "symbolId" in call.input)).toHaveLength(8);
    expect(calls.every((call) => call.name === "get_repository_context" || call.name === "search_code"
      ? true
      : call.name === "get_historical_changes" ? !call.input.symbolId : call.input.symbolId === "entity-1")).toBe(true);
  });

  it("does not repeat the full tool suite on follow-up turns", () => {
    expect(initialEvidenceToolCalls("And the tests?", "entity-1", false)).toEqual([
      { name: "search_code", input: { query: "test", limit: 20 } },
      { name: "analyze_change", input: { symbolId: "entity-1" } },
    ]);
  });

  it("requires the Rust-selected entity before gathering first-run evidence", () => {
    expect(() => initialEvidenceToolCalls("Question", undefined, true)).toThrow(/selected graph entity/);
  });
});
