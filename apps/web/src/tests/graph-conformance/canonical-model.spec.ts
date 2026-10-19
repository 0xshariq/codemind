import { describe, expect, it } from "vitest";
import { validateCanonicalResult } from "../../lib/server/results/validate";

const context = {
  workspaceId: "workspace-a",
  repositoryId: "repo-a",
  sourceSnapshotId: "snapshot-a",
  graphVersion: "graph-a",
};

function result(outcome: string, completeness = "complete", freshness = "current") {
  return {
    outcome,
    completeness,
    freshness,
    context,
    correlation: { requestId: "request-a", traceId: "trace-a", operationId: "operation-a" },
    data: { selected_entity_id: "symbol-a", impact_paths: [], test_candidates: [] },
    evidence: [{ path: "src/lib.rs", startLine: 4, endLine: 6, snapshotId: "snapshot-a" }],
    diagnostics: [{ code: "supported", message: "Source was parsed." }],
    limitations: [],
  };
}

describe("canonical graph result conformance", () => {
  it("preserves context, correlation, evidence, diagnostics and bounded result data", () => {
    const canonical = result("success");
    expect(() => validateCanonicalResult(canonical)).not.toThrow();
    expect(canonical.context).toEqual(context);
    expect(canonical.evidence[0]).toEqual({ path: "src/lib.rs", startLine: 4, endLine: 6, snapshotId: "snapshot-a" });
  });

  it.each(["success", "empty", "partial", "unsupported", "unavailable", "invalid", "failed"])(
    "accepts the distinct %s outcome state",
    (outcome) => {
      expect(() => validateCanonicalResult(result(outcome))).not.toThrow();
    },
  );

  it("keeps partial, truncated, unresolved and stale states qualified", () => {
    expect(() => validateCanonicalResult({ ...result("partial", "truncated", "unknown"), limitations: ["Traversal reached its configured bound."] })).not.toThrow();
    expect(() => validateCanonicalResult({ ...result("partial", "incomplete"), data: { unresolved_relationships: ["edge-a"] } })).not.toThrow();
    expect(() => validateCanonicalResult({ ...result("partial", "incomplete", "stale"), data: { freshness: "stale" } })).not.toThrow();
    expect(() => validateCanonicalResult({ ...result("success"), completeness: "truncated" })).toThrow(/truncated.*partial/i);
    expect(() => validateCanonicalResult({ ...result("success"), completeness: "incomplete" })).toThrow(/incomplete.*success/i);
  });

  it("rejects evidence that is unsafe or belongs to another snapshot", () => {
    expect(() => validateCanonicalResult({ ...result("success"), evidence: [{ path: "../secret.rs", line: 1, snapshotId: "snapshot-a" }] })).toThrow(/evidence/i);
    expect(() => validateCanonicalResult({ ...result("success"), evidence: [{ path: "src/lib.rs", line: 1, snapshotId: "snapshot-b" }] })).toThrow(/evidence/i);
  });

  it("requires outcome, data, shared context identity and correlation identity", () => {
    expect(() => validateCanonicalResult({ data: null })).toThrow(/outcome/i);
    expect(() => validateCanonicalResult({ ...result("success"), context: { ...context, graphVersion: "" } })).toThrow(/graph_version/i);
    expect(() => validateCanonicalResult({ ...result("success"), correlation: { requestId: "", traceId: "trace", operationId: "operation" } })).toThrow(/request_id/i);
  });
});
