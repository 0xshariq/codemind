import { z } from "zod";
import { randomUUID } from "node:crypto";
import { NextResponse } from "next/server";
import { createConfiguredProvider } from "@/lib/server/agent/provider";
import { planInvestigationFromQuestion } from "@/lib/server/agent/target-resolver";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { investigateChange } from "@/lib/server/workflows/investigate-change";
import { bindOperationOwner, publishOperation, setOperationResult } from "@/lib/server/operations/service";
import { getRepositoryInvestigationSource } from "@/lib/server/repositories/service";
import { createConversation, getConversation, saveConversation, type Conversation } from "@/lib/server/conversations/service";
import { startInvestigation } from "@/lib/server/rust-client/client";
import { streamRustProgress } from "@/lib/server/rust-client/progress-stream";
import { rustProgressState } from "@/lib/server/operations/rust-progress";

const investigationRequestSchema = z.object({
  question: z.string().trim().min(1).max(4_000),
  repositoryId: z.string().trim().min(1).max(1_024),
  conversationId: z.string().trim().min(1).max(128).optional(),
}).strict();

export async function POST(request: Request) {
  let session;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ error: "Sign in to start an investigation." }, { status: 401 });
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "The request body must be valid JSON." }, { status: 400 });
  }

  const parsed = investigationRequestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: "Provide a question and repository to investigate." }, { status: 400 });
  }

  const requestedRepositoryId = parsed.data.repositoryId;
  if (!authorizeMemoryRequest(session, session.workspaceId, "repository")) {
    return NextResponse.json({ error: "You do not have access to this repository scope." }, { status: 403 });
  }

  let investigationSource;
  try {
    investigationSource = await getRepositoryInvestigationSource(session.workspaceId, requestedRepositoryId);
  } catch {
    return NextResponse.json({ error: "Repository state is unavailable." }, { status: 502 });
  }
  if (!investigationSource) return NextResponse.json({ error: "Repository was not found or has no captured source files in this workspace." }, { status: 404 });
  const { repository, sourcePath, sourcePaths } = investigationSource;
  if (!repository.sourceSnapshotId || !repository.graphVersion) {
    return NextResponse.json({ error: "The selected repository has no canonical source snapshot yet." }, { status: 409 });
  }
  const authorizedContext = {
    repositoryId: repository.repositoryId,
    workspaceId: session.workspaceId,
    sourceSnapshotId: repository.sourceSnapshotId,
    graphVersion: repository.graphVersion,
  };

  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) {
    return NextResponse.json({ error: "Conversation scope is not authorized." }, { status: 403 });
  }
  let conversation: Conversation | null;
  if (parsed.data.conversationId) {
    conversation = await getConversation({ session, context: authorizedContext, conversationId: parsed.data.conversationId }).catch(() => null);
    if (!conversation) return NextResponse.json({ error: "Conversation was not found in this repository snapshot." }, { status: 404 });
  } else {
    conversation = await createConversation({ session, context: authorizedContext, title: `Investigation · ${repository.name}` }).catch(() => null);
    if (!conversation) return NextResponse.json({ error: "A saved conversation could not be created." }, { status: 502 });
  }

  const questionMessage = {
    id: randomUUID(),
    role: "user" as const,
    content: parsed.data.question,
    createdAt: new Date().toISOString(),
  };
  try {
    conversation = await saveConversation({
      session,
      conversation: {
        ...conversation!,
        messages: [...conversation!.messages, questionMessage],
        updatedAt: questionMessage.createdAt,
      },
    });
  } catch {
    return NextResponse.json({ error: "The question could not be saved to its conversation." }, { status: 502 });
  }

  const operationId = `web:${randomUUID()}`;
  bindOperationOwner(operationId, { userId: session.userId, workspaceId: session.workspaceId });
  publishOperation({ operationId, status: "planning", sequence: 1, message: "Question saved · planning the investigation" });

  void (async () => {
    let sequence = 1;
    let canonicalResult: unknown;
    let planningIssue: string | undefined;
    let phase = "plan question and title";
    let activeConversation = conversation!;
    const gatherAllTools = !activeConversation.initialEvidenceGathered;
    try {
      const provider = createConfiguredProvider();
      let selectedEntityId: string | undefined;
      if (activeConversation.graphInitialized && activeConversation.selectedEntityId) {
        selectedEntityId = activeConversation.selectedEntityId;
        sequence += 1;
        publishOperation({ operationId, status: "coie", sequence, message: "Using the existing repository graph" });
      } else {
        const plan = await planInvestigationFromQuestion(provider, parsed.data.question, sourcePaths);
        activeConversation = await saveConversation({
          session,
          conversation: { ...activeConversation, title: plan.title, updatedAt: new Date().toISOString() },
        });
        sequence += 1;
        publishOperation({ operationId, status: "planning", sequence, message: "Investigation title generated · resolving the engine entry point" });

        phase = "start Rust investigation";
        const started = await startInvestigation({ sourcePath, symbolName: plan.targetName, context: authorizedContext });
        const engineOperationId = typeof started.operationId === "string" ? started.operationId : "";
        selectedEntityId = typeof started.selectedEntityId === "string" ? started.selectedEntityId : "";
        const startedContext = started.context && typeof started.context === "object" ? started.context as Record<string, unknown> : {};
        const sameContext = (startedContext.repositoryId ?? startedContext.repository_id) === authorizedContext.repositoryId
          && (startedContext.workspaceId ?? startedContext.workspace_id) === authorizedContext.workspaceId
          && (startedContext.sourceSnapshotId ?? startedContext.source_snapshot_id) === authorizedContext.sourceSnapshotId
          && (startedContext.graphVersion ?? startedContext.graph_version) === authorizedContext.graphVersion;
        if (!engineOperationId || !selectedEntityId || !sameContext || engineOperationId.length > 1_024 || selectedEntityId.length > 1_024) {
          throw new Error("engine returned an invalid investigation reference");
        }

        let engineFailed = false;
        phase = "stream Rust engine progress";
        for await (const progress of streamRustProgress({ operationId: engineOperationId, afterSequence: 0, context: authorizedContext })) {
          if (progress.stageState === "failed") engineFailed = true;
          sequence += 1;
          publishOperation(rustProgressState(operationId, sequence, progress));
        }
        if (engineFailed) throw new Error("engine pipeline failed");
        // Persist graph readiness before semantic explanation. If the LLM or a
        // bounded evidence query fails, the next prompt reuses this graph.
        activeConversation = await saveConversation({
          session,
          conversation: { ...activeConversation, graphInitialized: true, selectedEntityId, updatedAt: new Date().toISOString() },
        });
        sequence += 1;
        publishOperation({ operationId, status: "chie", sequence, message: "Graph construction finished; analyzing canonical evidence" });
      }
      phase = "Agent Runtime semantic evidence query";
      const answer = await investigateChange(
        {
          provider,
          question: parsed.data.question,
          gatherAllTools,
          session,
          context: authorizedContext,
          ...(selectedEntityId ? { symbolId: selectedEntityId } : {}),
          conversationHistory: activeConversation.messages.slice(0, -1),
          onPlanningStart: () => {
            phase = "Agent Runtime follow-up tool planning";
          },
          onPlanningIssue: (reason) => {
            planningIssue = reason;
          },
          onToolStart: (name) => {
          phase = `Agent Runtime Rust semantic tool ${name}`;
          sequence += 1;
          const status = name === "get_test_intelligence" ? "tie" : name === "analyze_change" || name === "get_impact" ? "chie" : "coie";
          publishOperation({ operationId, status, sequence, message: `Running ${status.toUpperCase()} evidence stage` });
          },
          onReasoningStart: () => {
          phase = "Agent Runtime final model explanation";
          sequence += 1;
          publishOperation({ operationId, status: "reasoning", sequence, message: "Preparing evidence-grounded response" });
          },
          onCanonicalResult: (result) => {
          const value = result && typeof result === "object" ? result as Record<string, unknown> : {};
          const data = value.data && typeof value.data === "object" ? value.data as Record<string, unknown> : {};
          if (typeof data.selected_entity_id === "string" && Array.isArray(data.impact_paths)) canonicalResult = result;
          },
        },
      );
      sequence += 1;
      phase = "persist investigation result";
      setOperationResult(operationId, { answer, canonical: canonicalResult });
      if (activeConversation) {
        const now = new Date().toISOString();
        await saveConversation({
          session,
          conversation: {
            ...activeConversation,
            initialEvidenceGathered: activeConversation.initialEvidenceGathered === true || gatherAllTools,
            messages: [
              ...activeConversation.messages,
              { id: crypto.randomUUID(), role: "assistant", content: answer, createdAt: now },
            ],
            updatedAt: now,
          },
        }).catch(() => undefined);
      }
      publishOperation({
        operationId,
        status: planningIssue ? "partial" : "completed",
        sequence,
        message: planningIssue
          ? "Investigation completed with partial evidence because follow-up evidence planning was invalid."
          : "Investigation completed",
      });
    } catch (reason) {
      const errorCode = reason && typeof reason === "object" && "code" in reason ? (reason as { code?: unknown }).code : undefined;
      const message = reason instanceof Error ? reason.message.replace(/[\u0000-\u001f\u007f]/g, " ").replace(/(token|api[_ -]?key|authorization)\s*[:=]\s*\S+/gi, "$1=[redacted]").slice(0, 240) : "unknown error";
      const publicMessage = message.includes("OPENROUTER_API_KEY")
        ? "OpenRouter is not configured. Add OPENROUTER_API_KEY to the web app environment, then retry."
        : message.includes("Configured language model request failed")
          ? `OpenRouter did not complete the request${message.match(/HTTP \d{3}/)?.[0] ? ` (${message.match(/HTTP \d{3}/)?.[0]})` : ""}. Check model availability and provider rate limits, then retry.`
          : message.includes("Rust RPC")
            ? `The Rust engine request failed${typeof errorCode === "number" ? ` (gRPC code ${errorCode})` : ""}. Check the Rust engine terminal logs, then retry.`
            : `The investigation failed during ${phase}. Check the Next.js and Rust terminal logs for details.`;
      console.error("CodeMind investigation failed", { operationId, phase, name: reason instanceof Error ? reason.name : "unknown", code: errorCode, message });
      sequence += 1;
      setOperationResult(operationId, { error: publicMessage });
      publishOperation({ operationId, status: "failed", sequence, message: publicMessage });
    }
  })();

  return NextResponse.json({ operationId, conversationId: conversation.conversationId }, { status: 202 });
}
