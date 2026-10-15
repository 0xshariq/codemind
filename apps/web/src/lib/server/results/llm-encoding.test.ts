import { describe, expect, it } from "vitest";
import { decodeLlmResult, encodeLlmResult } from "./llm-encoding";

describe("bounded LLM encoding", () => {
  it("round trips nested values through flat and TOON", () => {
    const value = { context: { repositoryId: "repo" }, evidence: [{ path: "src/lib.rs" }] };
    expect(decodeLlmResult(encodeLlmResult(value))).toEqual(value);
  });

  it("rejects ambiguous object keys instead of silently changing evidence", () => {
    expect(() => encodeLlmResult({ "literal.key": "value" })).toThrow(/ambiguous key/);
  });

  it("preserves nulls and empty arrays", () => {
    const value = { evidence: [], finding: null };
    expect(decodeLlmResult(encodeLlmResult(value))).toEqual(value);
  });

  it("fails when semantic trimming cannot satisfy the byte budget", () => {
    expect(() => encodeLlmResult({ message: "x".repeat(100) }, 10)).toThrow(/budget/);
  });
});