import { describe, expect, it } from "vitest";
import { projectForUi } from "../../lib/server/results/ui-projection";

describe("relationship result semantics", () => {
  it("preserves edge type, direction, source evidence and result provenance", () => {
    const canonical = {
      outcome: "partial",
      completeness: "incomplete",
      freshness: "current",
      context: { repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph-v2" },
      correlation: { requestId: "request", traceId: "trace", operationId: "operation" },
      data: {
        relationships: [
          { kind: "CALLS", from: "caller", to: "callee", direction: "reverse" },
          { kind: "IMPORTS", from: "app.ts", to: "module.ts", direction: "forward" },
        ],
      },
      evidence: [{ path: "src/caller.rs", startLine: 8, endLine: 8, snapshotId: "snapshot" }],
      diagnostics: [{ code: "unresolved", message: "One import target is unresolved.", path: "src/unknown.ts" }],
      limitations: ["One relationship is unresolved."],
    };

    const projected = projectForUi(canonical);
    expect(projected.data).toEqual(canonical.data);
    expect(projected.data).toMatchObject({ relationships: [
      { kind: "CALLS", from: "caller", to: "callee", direction: "reverse" },
      { kind: "IMPORTS", from: "app.ts", to: "module.ts", direction: "forward" },
    ] });
    expect(projected.context).toEqual(canonical.context);
    expect(projected.completeness).toBe("incomplete");
    expect(projected.evidence).toEqual(canonical.evidence);
    expect(projected.diagnostics).toEqual(canonical.diagnostics);
    expect(projected.limitations).toEqual(canonical.limitations);
  });

  it("normalizes malformed optional diagnostics without changing edge facts", () => {
    const projected = projectForUi({
      outcome: "success",
      data: { relationships: [{ kind: "DEFINES", from: "file", to: "symbol" }] },
      diagnostics: ["parse warning", null, { message: "Supported note." }, { ignored: true }],
    });
    expect(projected.diagnostics).toEqual(["parse warning", { message: "Supported note.", code: undefined, path: undefined, severity: undefined }]);
    expect(projected.data).toEqual({ relationships: [{ kind: "DEFINES", from: "file", to: "symbol" }] });
  });

  it("rejects values that are not canonical relationship results", () => {
    expect(() => projectForUi(null)).toThrow(/canonical result/i);
    expect(() => projectForUi({ outcome: "success" })).toThrow(/outcome or data/i);
  });
});
