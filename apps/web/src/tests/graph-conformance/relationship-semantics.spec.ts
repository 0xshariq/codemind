import { describe, expect, it } from "vitest";
import { projectForUi } from "../../lib/server/results/ui-projection";

describe("relationship result semantics", () => {
  it("keeps structured relationship data for the UI", () => {
    const result = { outcome: "success", data: { relationship: "CALLS" } };
    expect(projectForUi(result)).toEqual(result);
  });
});