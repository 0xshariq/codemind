import { describe, expect, it } from "vitest";
import { decodeLlmResult, encodeLlmResult } from "./llm-encoding";

describe("bounded TOON LLM encoding", () => {
  it("round trips nested objects, arrays of records and evidence path identity", () => {
    const value = {
      context: { repositoryId: "repo", sourceSnapshotId: "snapshot", graphVersion: "graph" },
      result: { impact: { classification: "indirect", paths: [{ entityId: "caller", relationships: [{ kind: "CALLS", from: "caller", to: "target" }] }] } },
      evidence: [{ path: "src/lib.rs", startLine: 10, endLine: 12, snapshotId: "snapshot" }],
    };
    const encoded = encodeLlmResult(value);
    expect(typeof encoded).toBe("string");
    expect(encoded.length).toBeGreaterThan(0);
    expect(decodeLlmResult(encoded)).toEqual(value);
  });

  it("keeps mixed arrays as arrays through the flatten and unflatten boundary", () => {
    const value = { values: ["file.rs", 3, null, { path: "src/main.rs", relationships: ["CALLS", { kind: "IMPORTS" }] }] };
    const restored = decodeLlmResult(encodeLlmResult(value));
    expect(Array.isArray(restored.values)).toBe(true);
    expect(restored).toEqual(value);
  });

  it.each(["literal.key", "entry[0]", "constructor", "__proto__", "bad\nkey"])(
    "rejects ambiguous flattened object key %s",
    (key) => {
      expect(() => encodeLlmResult({ [key]: "value" })).toThrow(/ambiguous key/);
    },
  );

  it("preserves null and empty collection values", () => {
    const value = { evidence: [], finding: null, paths: {} };
    expect(decodeLlmResult(encodeLlmResult(value))).toEqual(value);
  });

  it("truncates large bounded result arrays and labels the TOON output partial", () => {
    const value = {
      outcome: "success",
      completeness: "complete",
      paths: Array.from({ length: 80 }, (_, index) => ({ id: `path-${index}`, path: `src/${index}.rs`, relationship: "CALLS" })),
    };
    const encoded = encodeLlmResult(value, 4_000);
    const restored = decodeLlmResult(encoded);
    expect(restored.outcome).toBe("partial");
    expect(restored.completeness).toBe("partial");
    expect(restored.truncated).toBe(true);
    expect((restored.paths as unknown[]).length).toBeLessThan(value.paths.length);
    expect(Buffer.byteLength(encoded, "utf8")).toBeLessThanOrEqual(4_000);
  });

  it("fails when a result with long scalar content cannot fit its byte budget", () => {
    expect(() => encodeLlmResult({ message: "x".repeat(100) }, 10)).toThrow(/budget/);
  });
});
