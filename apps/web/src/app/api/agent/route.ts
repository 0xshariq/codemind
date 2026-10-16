import { NextResponse } from "next/server";
import { createConfiguredProvider } from "@/lib/server/agent/provider";
import { investigateChange } from "@/lib/server/workflows/investigate-change";
import { publishOperation } from "@/lib/server/operations/service";
import { setOperationResult } from "@/lib/server/operations/service";
import { finalizeReview } from "@/lib/server/workflows/finalize-review";
import { loadHandoff, saveHandoff, updateHandoff } from "@/lib/server/handoffs/service";
import { persistMemory } from "@/lib/server/rust-client/client";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";

export async function POST(request: Request) {
  let body: {
    question?: unknown;
    toolCalls?: unknown;
    context?: {
      repositoryId?: unknown;
      workspaceId?: unknown;
      sourceSnapshotId?: unknown;
      graphVersion?: unknown;
    };
  };
  try {
    body = (await request.json()) as typeof body;
  } catch {
    return NextResponse.json({ error: "invalid JSON request body" }, { status: 400 });
  }
  if (typeof body.question !== "string" || body.question.trim().length === 0) {
    return NextResponse.json({ error: "question is required" }, { status: 400 });
  }
  const operationId = crypto.randomUUID();
  const toolCalls = Array.isArray(body.toolCalls)
    ? body.toolCalls.filter(
        (call): call is { name: string; input: unknown } =>
          !!call &&
          typeof call === "object" &&
          typeof (call as { name?: unknown }).name === "string" &&
          "input" in call,
      )
    : [];
  publishOperation({ operationId, status: "planning", sequence: 1, message: "Planning investigation" });
  void (async () => {
    let sequence = 1;
    try {
      const answer = await investigateChange(
        createConfiguredProvider(),
        body.question as string,
        toolCalls,
        (name) => {
          const stage = name === "get_test_intelligence" ? "tie" : name === "analyze_change" || name === "get_impact" ? "chie" : "coie";
          sequence += 1;
          publishOperation({ operationId, status: stage, sequence, message: stage === "coie" ? "Collecting canonical code evidence" : stage === "tie" ? "Collecting test intelligence" : "Computing change impact" });
        },
        () => {
          sequence += 1;
          publishOperation({ operationId, status: "reasoning", sequence, message: "Preparing evidence-grounded response" });
        },
      );
      sequence += 1;
      const context = body.context;
      const hasContext = context
        && Object.values(context).every((value) => typeof value === "string" && value.length > 0);
      let handoffResult: { handoffId?: string; handoffStatus?: string; targetStage?: string } = {};
      if (hasContext) {
        loadAuthenticatedSession(request);
        const reviewContext = {
          repositoryId: context.repositoryId as string,
          workspaceId: context.workspaceId as string,
          sourceSnapshotId: context.sourceSnapshotId as string,
          graphVersion: context.graphVersion as string,
        };
        const now = new Date().toISOString();
        await finalizeReview({
          contextId: operationId,
          scope: "repository",
          context: reviewContext,
          session: {
            sessionId: operationId,
            scope: "repository",
            context: reviewContext,
            startedAt: now,
            endedAt: now,
          },
          turns: [{ turnId: `${operationId}-turn`, sessionId: operationId, role: "assistant", summary: answer.slice(0, 4000), context: reviewContext, createdAt: now }],
          episode: { episodeId: `${operationId}-episode`, sessionId: operationId, title: "Impact investigation", context: reviewContext, createdAt: now },
          findings: [{ findingId: `${operationId}-finding`, episodeId: `${operationId}-episode`, summary: answer.slice(0, 4000), evidence: [], context: reviewContext, createdAt: now }],
          updatedAt: now,
        });
        const memoryPayload = JSON.stringify({ summary: answer.slice(0, 4000), operationId });
        for (const kind of ["semantic", "procedure"] as const) {
          await persistMemory({
            memoryId: `${operationId}-${kind}`,
            ownerId: "",
            kind,
            scope: "repository",
            lifecycle: "candidate",
            freshness: "fresh",
            payloadJson: memoryPayload,
            context: reviewContext,
            evidence: [],
            idempotencyKey: `${operationId}-${kind}`,
            version: 1,
            createdAt: now,
            updatedAt: now,
          });
        }
        await saveHandoff({
          handoffId: `${operationId}-handoff`,
          schemaVersion: "1",
          operationId,
          reviewEpisodeId: `${operationId}-episode`,
          sourceStage: "impact",
          targetStage: "test-investigation",
          task: body.question as string,
          findings: [answer.slice(0, 4000)],
          evidenceReferences: [],
          context: reviewContext,
          openQuestions: [],
          status: "pending",
          idempotencyKey: `${operationId}-handoff`,
          version: 1,
          createdAt: now,
          updatedAt: now,
        });
        const handoffId = `${operationId}-handoff`;
        await loadHandoff(handoffId, reviewContext);
        await updateHandoff(handoffId, "claimed", reviewContext);
        const consumed = await updateHandoff(handoffId, "consumed", reviewContext);
        const savedHandoff = consumed as { handoff?: { handoffId?: string } };
        handoffResult = { handoffId: String(savedHandoff.handoff?.handoffId ?? handoffId), handoffStatus: "consumed", targetStage: "test-investigation" };
      }
      setOperationResult(operationId, { answer, ...handoffResult });
      publishOperation({ operationId, status: "completed", sequence, message: "Investigation completed" });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Agent unavailable";
      sequence += 1;
      setOperationResult(operationId, { error: message });
      publishOperation({ operationId, status: "failed", sequence, message });
    }
  })();
  return NextResponse.json({ operationId }, { status: 202 });
}