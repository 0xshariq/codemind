import { z } from "zod";
import { randomUUID } from "node:crypto";
import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { runInvestigationOperation } from "@/lib/server/workflows/run-investigation-operation";
import { bindOperationOwner, publishOperation } from "@/lib/server/operations/service";
import { getRepositoryInvestigationSource } from "@/lib/server/repositories/service";
import { createConversation, getConversation, saveConversation, type Conversation } from "@/lib/server/conversations/service";

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

  void runInvestigationOperation({ operationId, session, question: parsed.data.question, context: authorizedContext, sourcePath, sourcePaths, conversation });

  return NextResponse.json({ operationId, conversationId: conversation.conversationId }, { status: 202 });
}
