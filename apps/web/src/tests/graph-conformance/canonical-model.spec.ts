import { describe, expect, it } from "vitest";
import { validateCanonicalResult } from "../../lib/server/results/validate";

describe("canonical result conformance", () => {
  it("requires an outcome and data", () => {
    expect(() => validateCanonicalResult({ data: null })).toThrow();
    expect(() => validateCanonicalResult({ outcome: "success", data: null })).not.toThrow();
  });
});