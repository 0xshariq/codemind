import "server-only";

import { randomUUID } from "node:crypto";
import { createConfiguredProvider } from "../agent/provider";
import { planInvestigationFromQuestion } from "../agent/target-resolver";
import { investigateChange } from "./investigate-change";
import { saveConversation, type Conversation } from "../conversations/service";
import { publishOperation, setOperationResult } from "../operations/service";
import { rustProgressState } from "../operations/rust-progress";
import { startInvestigation } from "../rust-client/client";
import { streamRustProgress } from "../rust-client/progress-stream";
import type { AuthenticatedSession } from "../auth/context";

type Context = { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };

function publicError(reason: unknown, phase: string) {
  const code = reason && typeof reason === "object" && "code" in reason ? (reason as { code?: unknown }).code : undefined;
  const message = reason instanceof Error ? reason.message.replace(/[\u0000-\u001f\u007f]/g, " ").replace(/(token|api[_ -]?key|authorization)\s*[:=]\s*\S+/gi, "$1=[redacted]").slice(0, 240) : "unknown error";
  if (message.includes("OPENROUTER_API_KEY")) return "OpenRouter is not configured. Add OPENROUTER_API_KEY to the web app environment, then retry.";
  if (message.includes("Configured language model request failed")) return `OpenRouter did not complete the request${message.match(/HTTP \d{3}/)?.[0] ? ` (${message.match(/HTTP \d{3}/)?.[0]})` : ""}. Check model availability and provider rate limits, then retry.`;
  if (message.includes("Rust RPC")) return `The Rust engine request failed${typeof code === "number" ? ` (gRPC code ${code})` : ""}. Check the Rust engine terminal logs, then retry.`;
  return `The investigation failed during ${phase}. Check the Next.js and Rust terminal logs for details.`;
}

export async function runInvestigationOperation(input: { operationId: string; session: AuthenticatedSession; question: string; context: Context; sourcePath: string; sourcePaths: readonly string[]; conversation: Conversation }) {
  let sequence = 1;
  let canonicalResult: unknown;
  let planningIssue: string | undefined;
  let phase = "planning";
  let activeConversation = input.conversation;
  const gatherAllTools = !activeConversation.initialEvidenceGathered;
  try {
    const provider = createConfiguredProvider();
    let selectedEntityId: string | undefined;
    if (activeConversation.graphInitialized && activeConversation.selectedEntityId) {
      selectedEntityId = activeConversation.selectedEntityId;
      sequence += 1;
      publishOperation({ operationId: input.operationId, status: "coie", sequence, message: "Using the existing repository graph" });
    } else {
      const plan = await planInvestigationFromQuestion(provider, input.question, input.sourcePaths);
      activeConversation = await saveConversation({ session: input.session, conversation: { ...activeConversation, title: plan.title, updatedAt: new Date().toISOString() } });
      sequence += 1;
      publishOperation({ operationId: input.operationId, status: "planning", sequence, message: "Investigation title generated · resolving the engine entry point" });
      phase = "start Rust investigation";
      const started = await startInvestigation({ sourcePath: input.sourcePath, symbolName: plan.targetName, context: input.context });
      const engineOperationId = typeof started.operationId === "string" ? started.operationId : "";
      selectedEntityId = typeof started.selectedEntityId === "string" ? started.selectedEntityId : "";
      const startedContext = started.context && typeof started.context === "object" ? started.context as Record<string, unknown> : {};
      const sameContext = (startedContext.repositoryId ?? startedContext.repository_id) === input.context.repositoryId && (startedContext.workspaceId ?? startedContext.workspace_id) === input.context.workspaceId && (startedContext.sourceSnapshotId ?? startedContext.source_snapshot_id) === input.context.sourceSnapshotId && (startedContext.graphVersion ?? startedContext.graph_version) === input.context.graphVersion;
      if (!engineOperationId || !selectedEntityId || !sameContext || engineOperationId.length > 1_024 || selectedEntityId.length > 1_024) throw new Error("engine returned an invalid investigation reference");
      phase = "stream Rust engine progress";
      let engineFailed = false;
      for await (const progress of streamRustProgress({ operationId: engineOperationId, afterSequence: 0, context: input.context })) { if (progress.stageState === "failed") engineFailed = true; sequence += 1; publishOperation(rustProgressState(input.operationId, sequence, progress)); }
      if (engineFailed) throw new Error("engine pipeline failed");
      activeConversation = await saveConversation({ session: input.session, conversation: { ...activeConversation, graphInitialized: true, selectedEntityId, updatedAt: new Date().toISOString() } });
      sequence += 1;
      publishOperation({ operationId: input.operationId, status: "chie", sequence, message: "Graph construction finished; analyzing canonical evidence" });
    }
    phase = "Agent Runtime semantic evidence query";
    const answer = await investigateChange({ provider, question: input.question, gatherAllTools, session: input.session, context: input.context, ...(selectedEntityId ? { symbolId: selectedEntityId } : {}), conversationHistory: activeConversation.messages.slice(0, -1), onPlanningStart: () => { phase = "Agent Runtime follow-up tool planning"; }, onPlanningIssue: (reason) => { planningIssue = reason; }, onToolStart: (name) => { phase = `Agent Runtime Rust semantic tool ${name}`; sequence += 1; const status = name === "get_test_intelligence" ? "tie" : name === "analyze_change" || name === "get_impact" ? "chie" : "coie"; publishOperation({ operationId: input.operationId, status, sequence, message: `Running ${status.toUpperCase()} evidence stage` }); }, onReasoningStart: () => { phase = "Agent Runtime final model explanation"; sequence += 1; publishOperation({ operationId: input.operationId, status: "reasoning", sequence, message: "Preparing evidence-grounded response" }); }, onCanonicalResult: (result) => { const value = result && typeof result === "object" ? result as Record<string, unknown> : {}; const data = value.data && typeof value.data === "object" ? value.data as Record<string, unknown> : {}; if (typeof data.selected_entity_id === "string" && Array.isArray(data.impact_paths)) canonicalResult = result; } });
    sequence += 1;
    setOperationResult(input.operationId, { answer, canonical: canonicalResult });
    const now = new Date().toISOString();
    await saveConversation({ session: input.session, conversation: { ...activeConversation, initialEvidenceGathered: activeConversation.initialEvidenceGathered === true || gatherAllTools, messages: [...activeConversation.messages, { id: randomUUID(), role: "assistant", content: answer, createdAt: now }], updatedAt: now } }).catch(() => undefined);
    publishOperation({ operationId: input.operationId, status: planningIssue ? "partial" : "completed", sequence, message: planningIssue ? "Investigation completed with partial evidence because follow-up evidence planning was invalid." : "Investigation completed" });
  } catch (reason) {
    const message = publicError(reason, phase);
    console.error("CodeMind investigation failed", { operationId: input.operationId, phase, name: reason instanceof Error ? reason.name : "unknown" });
    sequence += 1;
    setOperationResult(input.operationId, { error: message });
    publishOperation({ operationId: input.operationId, status: "failed", sequence, message });
  }
}
