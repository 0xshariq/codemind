import { describe, expect, it } from "vitest";
import { checkGuardrails, createGuardrailState } from "./guardrails";

describe("semantic tool guardrails", () => {
  it("rejects duplicate calls", () => {
    const state = createGuardrailState();
    checkGuardrails(state, "query:one", "query", 3);
    expect(() => checkGuardrails(state, "query:one", "query", 3)).toThrow(/duplicate/);
  });

  it("rejects calls after the shared budget", () => {
    const state = createGuardrailState();
    for (let index = 0; index < 20; index += 1) checkGuardrails(state, `query:${index}`, "query", 20);
    expect(() => checkGuardrails(state, "query:overflow", "query", 20)).toThrow(/budget/);
  });
});
