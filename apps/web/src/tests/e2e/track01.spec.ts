import { describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ analyzeChange: vi.fn(), getRepository: vi.fn() }));
vi.mock("server-only", () => ({}));
vi.mock("../../lib/server/rust-client/client", () => ({ analyzeChange: mocks.analyzeChange }));
vi.mock("../../lib/server/repositories/service", () => ({ getRepository: mocks.getRepository }));

import { runAgent } from "../../lib/server/agent/runtime";
import type { AgentProvider } from "../../lib/server/agent/provider";
import { projectForUi } from "../../lib/server/results/ui-projection";
import { validateCanonicalResult } from "../../lib/server/results/validate";
import { authorizeRepositoryContext } from "../../lib/server/auth/authorize";

const context = { repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph" };

function rustResponse(contextValue = context) {
  return {
    data: {
      outcome: 1,
      completeness: 1,
      freshness: 1,
      context: contextValue,
      correlation: { requestId: "request", traceId: "trace", operationId: "operation" },
      dataJson: JSON.stringify({
        selected_entity_id: "entity-file",
        impact_paths: [{
          entity_id: "caller-symbol",
          classification: "direct",
          depth: 1,
          relationships: [{ kind: "CALLS", from: "caller-symbol", to: "entity-file", direction: "reverse" }],
        }],
        risk: "unknown",
        test_candidates: [],
      }),
      evidence: [{ path: "src/main.rs", startLine: 3, endLine: 5, snapshotId: "snapshot" }],
      diagnostics: [],
      limitations: [],
    },
  };
}

const session = {
  version: 1 as const,
  sessionId: "session",
  userId: "user",
  workspaceId: "workspace",
  allowedScopes: ["repository"],
  teamIds: [],
  issuedAt: 1,
  expiresAt: 100,
};

describe("Track 01 semantic investigation server integration", () => {
  it("runs question → authorized tool → Rust gRPC result → canonical UI projection", async () => {
    mocks.analyzeChange.mockResolvedValueOnce(rustResponse());
    const provider: AgentProvider = {
      complete: vi.fn(async (_messages, systemPrompt) => systemPrompt?.includes("Tool planning mode") ? '{"toolCalls":[]}' : "Grounded explanation."),
    };
    const canonicalValues: unknown[] = [];
    const answer = await runAgent(
      provider,
      "What could changing src/main.rs affect?",
      [{ name: "analyze_change", input: { maxDepth: 3 } }],
      undefined,
      undefined,
      (value) => canonicalValues.push(value),
      {
        context,
        symbolId: "entity-file",
        authorizeTool: async (_name, input) => {
          const toolContext = (input as { context: typeof context }).context;
          if (JSON.stringify(toolContext) !== JSON.stringify(context)) throw new Error("cross-snapshot tool call denied");
        },
      },
    );

    expect(answer).toBe("Grounded explanation.");
    expect(mocks.analyzeChange).toHaveBeenCalledWith({ symbolId: "entity-file", maxDepth: 3, context });
    expect(canonicalValues).toHaveLength(1);
    expect(() => validateCanonicalResult(canonicalValues[0])).not.toThrow();
    const uiResult = projectForUi(canonicalValues[0]);
    expect(uiResult.data).toMatchObject({ impact_paths: [{ entity_id: "caller-symbol", relationships: [{ kind: "CALLS", from: "caller-symbol", to: "entity-file" }] }] });
    expect(uiResult.evidence).toEqual([{ path: "src/main.rs", startLine: 3, endLine: 5, snapshotId: "snapshot" }]);
    expect(uiResult.context).toMatchObject({ repositoryId: "repo", sourceSnapshotId: "snapshot", graphVersion: "graph" });
  });

  it("rejects a Rust response bound to another repository snapshot", async () => {
    mocks.analyzeChange.mockResolvedValueOnce(rustResponse({ ...context, sourceSnapshotId: "other-snapshot" }));
    const provider: AgentProvider = { complete: async () => '{"toolCalls":[]}' };
    await expect(runAgent(provider, "Analyze src/main.rs", [{ name: "analyze_change", input: { maxDepth: 3 } }], undefined, undefined, undefined, {
      context,
      symbolId: "entity-file",
      authorizeTool: async () => undefined,
    })).rejects.toThrow(/context does not match/);
  });

  it("keeps the workspace and repository authorization boundary explicit", async () => {
    mocks.getRepository.mockResolvedValue({ repositoryId: "repo", sourceSnapshotId: "snapshot", graphVersion: "graph" });
    await expect(authorizeRepositoryContext(session, context)).resolves.toBe(true);
    await expect(authorizeRepositoryContext(session, { ...context, sourceSnapshotId: "other" })).resolves.toBe(false);
    await expect(authorizeRepositoryContext({ ...session, allowedScopes: ["user"] }, context)).resolves.toBe(false);
  });
});
