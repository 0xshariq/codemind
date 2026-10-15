import { describe, expect, it, vi } from "vitest";
vi.mock("server-only", () => ({}));
import { runAgent } from "./runtime";

describe("bounded agent runtime", () => {
  it("passes evidence to the configured provider", async () => {
    const result = await runAgent(
      { complete: async (messages) => messages.at(-1)?.content ?? "" },
      "What could this affect?",
    );
    expect(result).toContain("What could this affect?");
  });
});
