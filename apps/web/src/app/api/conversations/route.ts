import { NextResponse } from "next/server";
import { z } from "zod";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { createConversation, listConversations } from "@/lib/server/conversations/service";

const contextSchema = z.object({
  repositoryId: z.string().trim().min(1).max(1_024),
  sourceSnapshotId: z.string().trim().min(1).max(1_024),
  graphVersion: z.string().trim().min(1).max(1_024),
}).strict();
const createSchema = z.object({ context: contextSchema, title: z.string().trim().max(160).optional() }).strict();

export async function GET(request: Request) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to view conversations." }, { status: 401 }); }

  const url = new URL(request.url);
  const parsed = contextSchema.safeParse(Object.fromEntries(
    ["repositoryId", "sourceSnapshotId", "graphVersion"].map((key) => [key, url.searchParams.get(key) ?? ""]),
  ));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: "Repository context is required." }, { status: 400 });
  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) {
    return NextResponse.json({ outcome: "forbidden", error: "Conversation scope is not authorized." }, { status: 403 });
  }
  try {
    const conversations = await listConversations({ session, context: { ...parsed.data, workspaceId: session.workspaceId } });
    return NextResponse.json({ outcome: conversations.length ? "success" : "empty", conversations }, { headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Conversation storage is unavailable." }, { status: 502 });
  }
}

export async function POST(request: Request) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to create a conversation." }, { status: 401 }); }
  const parsed = createSchema.safeParse(await request.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: "A valid repository context and optional title are required." }, { status: 400 });
  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) {
    return NextResponse.json({ outcome: "forbidden", error: "Conversation scope is not authorized." }, { status: 403 });
  }
  try {
    const conversation = await createConversation({ session, context: { ...parsed.data.context, workspaceId: session.workspaceId }, title: parsed.data.title });
    return NextResponse.json({ outcome: "success", conversation }, { status: 201, headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Conversation could not be created." }, { status: 502 });
  }
}
