import { beforeEach, describe, expect, it, vi } from "vitest";
import { encodeLlmResult } from "../results/llm-encoding";

const mocks = vi.hoisted(() => ({ executeToolLoop: vi.fn() }));

vi.mock("server-only", () => ({}));
vi.mock("./tool-loop", () => ({ executeToolLoop: mocks.executeToolLoop }));

import { runAgent } from "./runtime";
import { planInvestigationFromQuestion, targetFromQuestion } from "./target-resolver";

const context = { repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph" };
const execution = {
  context,
  symbolId: "entity-file",
  authorizeTool: vi.fn(async () => undefined),
};
const canonical = {
  outcome: "success",
  completeness: "complete",
  freshness: "current",
  context,
  correlation: { requestId: "request", traceId: "trace", operationId: "operation" },
  data: { selected_entity_id: "entity-file", impact_paths: [], test_candidates: [] },
  evidence: [],
  diagnostics: [],
  limitations: [],
};

function makeProvider(plan = '{"toolCalls":[]}') {
  return {
    complete: vi.fn(async (messages: readonly { role: string; content: string }[]) =>
      messages.some((message) => message.content.startsWith("Select only the semantic tools")) ? plan : "Evidence-backed explanation."),
  };
}

function mockToolExecutor(result: Record<string, unknown> = canonical) {
  mocks.executeToolLoop.mockImplementation(async (calls, budget, onToolStart, onCanonicalResult, authorize) => {
    const seen = new Set<string>();
    const values: string[] = [];
    for (const call of calls.slice(0, budget)) {
      const key = `${call.name}:${JSON.stringify(call.input)}`;
      if (seen.has(key)) throw new Error("agent tool loop stopped: duplicate tool call");
      seen.add(key);
      onToolStart?.(call.name);
      await authorize?.(call.name, call.input);
      onCanonicalResult?.(result);
      values.push(encodeLlmResult(result));
    }
    return values;
  });
}

describe("bounded agent runtime", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    execution.authorizeTool.mockReset();
    execution.authorizeTool.mockResolvedValue(undefined);
    mockToolExecutor();
  });

  it("injects the authorized repository and selected entity into model-planned tools", async () => {
    const provider = makeProvider();
    const onToolStart = vi.fn();
    const onCanonicalResult = vi.fn();
    const onPlanningStart = vi.fn();
    const answer = await runAgent(provider, "What could src/lib.rs affect?", [
      { name: "analyze_change", input: { maxDepth: 3 } },
    ], onToolStart, undefined, onCanonicalResult, execution, onPlanningStart);

    expect(answer).toBe("Evidence-backed explanation.");
    expect(mocks.executeToolLoop).toHaveBeenCalledTimes(1);
    expect(mocks.executeToolLoop.mock.calls[0][0][0].input).toEqual({ maxDepth: 3, context, symbolId: "entity-file" });
    expect(execution.authorizeTool).toHaveBeenCalledWith("analyze_change", { maxDepth: 3, context, symbolId: "entity-file" });
    expect(onToolStart).toHaveBeenCalledWith("analyze_change");
    expect(onCanonicalResult).toHaveBeenCalledWith(canonical);
    expect(onPlanningStart).toHaveBeenCalledOnce();
    expect(provider.complete.mock.calls.at(-1)?.[0].some((message) => message.role === "tool" && message.content.includes("selected_entity_id"))).toBe(true);
  });

  it("uses the completed JavaScript tool results directly without a fragile extra planning round", async () => {
    const provider = makeProvider("The previous tool-planning response was prose, not JSON.");
    const onPlanningStart = vi.fn();
    const answer = await runAgent(provider, "What happens when I add a project?", [
      { name: "search_code", input: { query: "project", limit: 20 } },
    ], undefined, undefined, undefined, { ...execution, preplannedOnly: true }, onPlanningStart);

    expect(answer).toBe("Evidence-backed explanation.");
    expect(provider.complete).toHaveBeenCalledOnce();
    expect(onPlanningStart).not.toHaveBeenCalled();
    expect(mocks.executeToolLoop).toHaveBeenCalledOnce();
  });

  it("keeps prior turns available to follow-up explanations in the same conversation", async () => {
    const provider = makeProvider();
    await runAgent(provider, "And what tests cover it?", [], undefined, undefined, undefined, {
      ...execution,
      conversationHistory: [
        { role: "user", content: "What changes if I update the engine?" },
        { role: "assistant", content: "The engine entry point coordinates the pipeline." },
      ],
    });
    const finalMessages = provider.complete.mock.calls.at(-1)?.[0] ?? [];
    expect(finalMessages).toEqual(expect.arrayContaining([
      expect.objectContaining({ role: "user", content: "What changes if I update the engine?" }),
      expect.objectContaining({ role: "assistant", content: "The engine entry point coordinates the pipeline." }),
    ]));
  });

  it("bounds calls and passes an explicit partial result to the explanation stage", async () => {
    mockToolExecutor({ ...canonical, outcome: "unsupported", completeness: "incomplete", data: { message: "Unsupported query" }, limitations: ["Unsupported by engine"] });
    const provider = makeProvider();
    const calls = Array.from({ length: 16 }, (_, index) => ({ name: "search_code", input: { query: `query-${index}` } }));
    await runAgent(provider, "Explain this source", calls, undefined, undefined, undefined, execution);
    expect(mocks.executeToolLoop.mock.calls[0][0]).toHaveLength(16);
    const finalMessages = provider.complete.mock.calls.at(-1)?.[0] ?? [];
    expect(finalMessages.some((message) => message.role === "tool" && message.content.includes("Unsupported by engine"))).toBe(true);
    expect(finalMessages.some((message) => message.role === "tool" && message.content.includes("truncated"))).toBe(true);
  });

  it("fails repeated identical calls instead of silently executing a duplicate", async () => {
    const provider = makeProvider();
    const duplicateCalls = [
      { name: "search_code", input: { query: "target" } },
      { name: "search_code", input: { query: "target" } },
    ];
    await expect(runAgent(provider, "Find target", duplicateCalls, undefined, undefined, undefined, execution)).rejects.toThrow(/duplicate/);
  });

  it("stops when the provider fails during planning", async () => {
    const provider = makeProvider();
    provider.complete.mockRejectedValueOnce(new Error("provider unavailable"));
    await expect(runAgent(provider, "Question", [{ name: "analyze_change", input: {} }], undefined, undefined, undefined, execution)).rejects.toThrow(/provider unavailable/);
  });

  it("turns malformed, oversized, and unregistered follow-up plans into partial evidence", async () => {
    const tooMany = makeProvider(JSON.stringify({ toolCalls: Array.from({ length: 17 }, () => ({ name: "search_code", input: { query: "x" } })) }));
    await expect(runAgent(tooMany, "Question", [], undefined, undefined, undefined, execution)).resolves.toBe("Evidence-backed explanation.");
    const unknownTool = makeProvider('{"toolCalls":[{"name":"run_shell","input":{}}]}');
    await expect(runAgent(unknownTool, "Question", [], undefined, undefined, undefined, execution)).resolves.toBe("Evidence-backed explanation.");
    const malformed = makeProvider("not JSON");
    const onPlanningIssue = vi.fn();
    await expect(runAgent(malformed, "Question", [], undefined, undefined, undefined, execution, undefined, onPlanningIssue)).resolves.toBe("Evidence-backed explanation.");
    expect(onPlanningIssue).toHaveBeenCalledWith("Agent returned an invalid tool plan");
    const finalMessages = malformed.complete.mock.calls.at(-1)?.[0] ?? [];
    expect(finalMessages.some((message) => message.role === "tool" && message.content.includes("could not be gathered"))).toBe(true);
  });

  it("keeps canonical evidence when follow-up tool planning returns malformed JSON", async () => {
    const provider = makeProvider("not JSON");
    const onPlanningIssue = vi.fn();
    await expect(runAgent(provider, "Question", [
      { name: "analyze_change", input: {} },
    ], undefined, undefined, undefined, execution, undefined, onPlanningIssue)).resolves.toBe("Evidence-backed explanation.");
    expect(onPlanningIssue).toHaveBeenCalledWith("Agent returned an invalid tool plan");
    const finalMessages = provider.complete.mock.calls.at(-1)?.[0] ?? [];
    expect(finalMessages.some((message) => message.role === "tool" && message.content.includes("selected_entity_id"))).toBe(true);
    expect(finalMessages.some((message) => message.role === "tool" && message.content.includes("could not be gathered"))).toBe(true);
  });

  it("executes the OpenRouter tool/arguments plan shape through the registered JavaScript tool", async () => {
    let planningRounds = 0;
    const provider = {
      complete: vi.fn(async (messages: readonly { role: string; content: string }[]) => {
        if (messages.some((message) => message.content.startsWith("Select only the semantic tools"))) {
          planningRounds += 1;
          return planningRounds === 1
            ? '{"tool":"search","arguments":{"query":"project","limit":20}}'
            : '{"toolCalls":[]}';
        }
        return "Evidence-backed explanation.";
      }),
    };

    await expect(runAgent(provider, "What does the project do?", [
      { name: "analyze_change", input: {} },
    ], undefined, undefined, undefined, execution)).resolves.toBe("Evidence-backed explanation.");

    const dispatchedCalls = mocks.executeToolLoop.mock.calls.flatMap((call) => call[0]);
    expect(dispatchedCalls).toContainEqual(expect.objectContaining({
      name: "search_code",
      input: expect.objectContaining({ query: "project", limit: 20, context }),
    }));
  });

  it("searches the graph first and scopes follow-up analysis to an entity returned by Rust", async () => {
    let planningRounds = 0;
    const discovered = "canonical-project-entity";
    const searchResult = { ...canonical, data: { query: "portfolio", matches: [{ id: discovered, name: "Portfolio", path: "src/portfolio.ts" }], truncated: false } };
    const provider = {
      complete: vi.fn(async (messages: readonly { role: string; content: string }[]) => {
        if (messages.some((message) => message.content.startsWith("Select only the semantic tools"))) {
          planningRounds += 1;
          return planningRounds === 1
            ? JSON.stringify({ toolCalls: [{ name: "search_code", input: { query: "portfolio", limit: 20 } }] })
            : planningRounds === 2
              ? JSON.stringify({ toolCalls: [{ name: "analyze_change", input: { symbolId: discovered, maxDepth: 3 } }] })
              : '{"toolCalls":[]}';
        }
        return "Evidence-backed explanation.";
      }),
    };
    mocks.executeToolLoop.mockImplementation(async (calls, _budget, onToolStart, onCanonicalResult, authorize) => {
      const call = calls[0];
      const result = call.name === "search_code" ? searchResult : canonical;
      onToolStart?.(call.name);
      await authorize?.(call.name, call.input);
      onCanonicalResult?.(result);
      return [encodeLlmResult(result)];
    });

    await expect(runAgent(provider, "What changes when adding projects to the portfolio?", [], undefined, undefined, undefined, execution))
      .resolves.toBe("Evidence-backed explanation.");
    const calls = mocks.executeToolLoop.mock.calls.flatMap((call) => call[0]);
    expect(calls[0]).toMatchObject({ name: "search_code", input: { query: "portfolio", context } });
    expect(calls[1]).toMatchObject({ name: "analyze_change", input: { symbolId: discovered, context, maxDepth: 3 } });
  });

  it("does not render a model tool request as the assistant answer", async () => {
    const provider = {
      complete: vi.fn(async (messages: readonly { role: string; content: string }[]) => {
        if (messages.some((message) => message.content.startsWith("Select only the semantic tools"))) return '{"toolCalls":[]}';
        if (messages.at(-1)?.content.includes("That was a tool request")) return "The repository manages portfolio projects.";
        return '{"tool":"search","arguments":{"query":"project","limit":20}}';
      }),
    };
    const onPlanningIssue = vi.fn();

    await expect(runAgent(provider, "What does the project do?", [], undefined, undefined, undefined, execution, undefined, onPlanningIssue))
      .resolves.toBe("The repository manages portfolio projects.");
    expect(onPlanningIssue).toHaveBeenCalledWith("The model returned a tool request instead of a user-facing explanation");
  });
});

describe("question target extraction", () => {
  const repositoryFiles = ["README.md", "engine/src/main.rs", "apps/web/src/app/page.tsx"];

  it("uses an explicitly named path when it belongs to the source manifest", async () => {
    const question = "What could changing src/lib/server/runtime.ts affect?";
    const files = [...repositoryFiles, "src/lib/server/runtime.ts"];
    await expect(targetFromQuestion({ complete: vi.fn() }, question, files)).resolves.toBe("src/lib/server/runtime.ts");
  });

  it("lets the model select a captured source file for a vague natural-language question", async () => {
    const provider = { complete: vi.fn(async () => '{"targetName":"engine/src/main.rs"}') };
    await expect(targetFromQuestion(provider, "What happens when I add a new engine in Rust?", repositoryFiles)).resolves.toBe("engine/src/main.rs");
    expect(provider.complete).toHaveBeenCalledOnce();
  });

  it("generates a concise conversation title with the selected source file", async () => {
    const provider = { complete: vi.fn(async () => '{"targetName":"engine/src/main.rs","title":"Adding a Rust engine"}') };
    await expect(planInvestigationFromQuestion(provider, "What happens when I add a new engine in Rust?", repositoryFiles)).resolves.toEqual({
      targetName: "engine/src/main.rs",
      title: "Adding a Rust engine",
    });
  });

  it("keeps vague questions usable and creates a fallback title when the model cannot plan", async () => {
    const plan = await planInvestigationFromQuestion({ complete: vi.fn(async () => "not json") }, "What happens when I add a new engine?", repositoryFiles);
    expect(repositoryFiles).toContain(plan.targetName);
    expect(plan.title).toBe("What happens when I add a new engine");
  });

  it("falls back to a manifest file when the model cannot select a target", async () => {
    await expect(targetFromQuestion({ complete: vi.fn(async () => "not json") }, "Tell me something", repositoryFiles)).resolves.toBe("engine/src/main.rs");
    await expect(targetFromQuestion({ complete: vi.fn(async () => { throw new Error("provider unavailable"); }) }, "Tell me something", repositoryFiles)).resolves.toBe("engine/src/main.rs");
  });

  it("rejects a model-selected path outside the repository manifest and falls back safely", async () => {
    const target = await targetFromQuestion({ complete: vi.fn(async () => '{"targetName":"../../secrets.env"}') }, "What changes?", repositoryFiles);
    expect(repositoryFiles).toContain(target);
    expect(target).not.toBe("../../secrets.env");
  });

  it.each(["../secret.rs", "/etc/passwd", "C:\\private\\file.rs", "src\\..\\secret.rs"])(
    "rejects unsafe target paths such as %s",
    async (targetName) => {
      const question = `Inspect ${targetName}`;
      await expect(targetFromQuestion({ complete: vi.fn() }, question, [targetName])).rejects.toThrow(/no usable source files/);
    },
  );
});
