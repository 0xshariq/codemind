import { describe, expect, it } from "vitest";
import { contextSchema } from "../../lib/server/tools/schemas";

describe("snapshot isolation", () => {
  it("requires source snapshot and graph version", () => {
    expect(contextSchema.safeParse({ repositoryId: "r", workspaceId: "w" }).success).toBe(false);
  });
});