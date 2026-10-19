import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ handler: vi.fn() }));
vi.mock("server-only", () => ({}));
vi.mock("./implementations", () => ({ TOOL_HANDLERS: {
  get_repository_context: mocks.handler,
  get_historical_changes: mocks.handler,
} }));

import { executeToolLoop } from "../agent/tool-loop";
import { accountToolResult, checkGuardrails, createGuardrailState, MAX_TOOL_RESULT_BYTES } from "./guardrails";
import { parseToolInput } from "./schemas";

const context = { repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph" };
const call = { name: "get_repository_context", input: { context } };

describe("semantic tool guardrails", () => {
  beforeEach(() => {
    mocks.handler.mockReset();
    mocks.handler.mockResolvedValue({ outcome: "success", data: { repositoryId: "repo" } });
  });

  it("rejects duplicate calls and calls after the global budget", () => {
    const state = createGuardrailState();
    checkGuardrails(state, "query:one", "search_code", 3);
    expect(() => checkGuardrails(state, "query:one", "search_code", 3)).toThrow(/duplicate/);
    const full = createGuardrailState();
    for (let index = 0; index < 20; index += 1) checkGuardrails(full, `query:${index}`, "search_code", 20);
    expect(() => checkGuardrails(full, "query:overflow", "search_code", 20)).toThrow(/budget/);
  });

  it("does not apply a wall-clock deadline and still enforces per-tool call limits", () => {
    const longRunning = createGuardrailState();
    expect(() => checkGuardrails(longRunning, "long-running", "search_code", 3)).not.toThrow();
    const limited = createGuardrailState();
    checkGuardrails(limited, "first", "search_code", 1);
    expect(() => checkGuardrails(limited, "second", "search_code", 1)).toThrow(/limit exhausted/);
  });

  it("keeps every tool call in one authorized repository snapshot", () => {
    const state = createGuardrailState();
    checkGuardrails(state, "repo-a", "search_code", 3, "workspace\0repo\0snapshot\0graph");
    expect(() => checkGuardrails(state, "repo-b", "search_code", 3, "workspace\0repo\0other-snapshot\0graph")).toThrow(/context budget/);
  });

  it("rejects oversized or malformed tool inputs before dispatch", () => {
    expect(() => parseToolInput("search_code", { context, query: "x".repeat(501) })).toThrow();
    expect(() => parseToolInput("search_code", { context: { ...context, repositoryId: "" }, query: "safe" })).toThrow();
    expect(parseToolInput("search_code", { context, query: "safe" })).toMatchObject({ context, query: "safe", limit: 20 });
  });

  it("enforces serialized result byte limits", () => {
    const state = createGuardrailState();
    expect(() => accountToolResult(state, "x".repeat(MAX_TOOL_RESULT_BYTES + 1))).toThrow(/result budget/);
    accountToolResult(state, JSON.stringify({ outcome: "success", data: null }));
    expect(state.resultBytes).toBeGreaterThan(0);
  });

  it("requires authorization before dispatch and passes authorized tool results through validation", async () => {
    await expect(executeToolLoop([call])).rejects.toThrow(/authorization/);
    expect(mocks.handler).not.toHaveBeenCalled();

    const authorization = vi.fn(async () => undefined);
    await expect(executeToolLoop([call], 8, undefined, undefined, authorization)).resolves.toHaveLength(1);
    expect(authorization).toHaveBeenCalledWith("get_repository_context", call.input);
    expect(mocks.handler).toHaveBeenCalledTimes(1);
  });

  it("retains the investigation when one authorized semantic tool fails", async () => {
    mocks.handler.mockRejectedValueOnce(new Error("symbol lookup is unsupported"));
    const authorization = vi.fn(async () => undefined);
    const results = await executeToolLoop(
      [call, { name: "get_historical_changes", input: { context, limit: 10 } }],
      8,
      undefined,
      undefined,
      authorization,
    );

    expect(results).toHaveLength(2);
    expect(results[0]).toContain("unavailable");
    expect(results[0]).toContain("other available evidence was retained");
    expect(results[1]).toContain("success");
    expect(authorization).toHaveBeenCalledTimes(2);
  });

  it("does not dispatch repeated calls after the first matching request", async () => {
    const authorization = vi.fn(async () => undefined);
    await expect(executeToolLoop([call, call], 8, undefined, undefined, authorization)).rejects.toThrow(/duplicate/);
    expect(mocks.handler).toHaveBeenCalledTimes(1);
  });
});
