import { NextResponse } from "next/server";
import { z } from "zod";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { deleteConversation, getConversation, saveConversation } from "@/lib/server/conversations/service";

const contextSchema = z.object({
  repositoryId: z.string().trim().min(1).max(1_024),
  sourceSnapshotId: z.string().trim().min(1).max(1_024),
  graphVersion: z.string().trim().min(1).max(1_024),
}).strict();
const updateSchema = z.object({
  context: contextSchema,
  title: z.string().trim().min(1).max(160),
  messages: z.array(z.object({
    id: z.string().trim().min(1).max(128),
    role: z.enum(["user", "assistant"]),
    content: z.string().max(4_000),
    createdAt: z.string().datetime(),
  }).strict()).max(100),
}).strict();

export async function GET(request: Request, { params }: { params: Promise<{ conversationId: string }> }) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to view this conversation." }, { status: 401 }); }
  const { conversationId } = await params;
  if (!conversationId.trim() || conversationId.length > 128) return NextResponse.json({ outcome: "invalid", error: "A valid conversation ID is required." }, { status: 400 });
  const url = new URL(request.url);
  const context = contextSchema.safeParse(Object.fromEntries(
    ["repositoryId", "sourceSnapshotId", "graphVersion"].map((key) => [key, url.searchParams.get(key) ?? ""]),
  ));
  if (!context.success) return NextResponse.json({ outcome: "invalid", error: "Repository context is required." }, { status: 400 });
  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) return NextResponse.json({ outcome: "forbidden", error: "Conversation scope is not authorized." }, { status: 403 });
  try {
    const conversation = await getConversation({ session, context: { ...context.data, workspaceId: session.workspaceId }, conversationId });
    if (!conversation) return NextResponse.json({ outcome: "not_found", error: "Conversation was not found." }, { status: 404 });
    return NextResponse.json({ outcome: "success", conversation }, { headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Conversation storage is unavailable." }, { status: 502 });
  }
}

export async function PATCH(request: Request, { params }: { params: Promise<{ conversationId: string }> }) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to update this conversation." }, { status: 401 }); }
  const { conversationId } = await params;
  if (!conversationId.trim() || conversationId.length > 128) return NextResponse.json({ outcome: "invalid", error: "A valid conversation ID is required." }, { status: 400 });
  const parsed = updateSchema.safeParse(await request.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: "Conversation update is invalid or exceeds its limits." }, { status: 400 });
  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) return NextResponse.json({ outcome: "forbidden", error: "Conversation scope is not authorized." }, { status: 403 });
  const context = { ...parsed.data.context, workspaceId: session.workspaceId };
  try {
    const existing = await getConversation({ session, context, conversationId });
    if (!existing) return NextResponse.json({ outcome: "not_found", error: "Conversation was not found." }, { status: 404 });
    const conversation = await saveConversation({
      session,
      conversation: {
        ...existing,
        title: parsed.data.title,
        messages: parsed.data.messages,
        updatedAt: new Date().toISOString(),
      },
    });
    return NextResponse.json({ outcome: "success", conversation }, { headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Conversation could not be updated." }, { status: 502 });
  }
}

export async function DELETE(request: Request, { params }: { params: Promise<{ conversationId: string }> }) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to delete this conversation." }, { status: 401 }); }
  const { conversationId } = await params;
  if (!conversationId.trim() || conversationId.length > 128) return NextResponse.json({ outcome: "invalid", error: "A valid conversation ID is required." }, { status: 400 });
  const url = new URL(request.url);
  const parsed = contextSchema.safeParse(Object.fromEntries(
    ["repositoryId", "sourceSnapshotId", "graphVersion"].map((key) => [key, url.searchParams.get(key) ?? ""]),
  ));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: "Repository context is required." }, { status: 400 });
  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) return NextResponse.json({ outcome: "forbidden", error: "Conversation scope is not authorized." }, { status: 403 });
  try {
    const deleted = await deleteConversation({ session, context: { ...parsed.data, workspaceId: session.workspaceId }, conversationId });
    return deleted
      ? NextResponse.json({ outcome: "success" }, { headers: { "cache-control": "no-store" } })
      : NextResponse.json({ outcome: "not_found", error: "Conversation was not found." }, { status: 404 });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Conversation could not be deleted." }, { status: 502 });
  }
}
