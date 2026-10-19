import { NextResponse } from "next/server";
import { z } from "zod";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { searchConversations } from "@/lib/server/conversations/service";

const querySchema = z.object({
  q: z.string().trim().min(1).max(200),
  repositoryId: z.string().trim().min(1).max(1_024),
  sourceSnapshotId: z.string().trim().min(1).max(1_024),
  graphVersion: z.string().trim().min(1).max(1_024),
  limit: z.coerce.number().int().min(1).max(50).default(20),
});

export async function GET(request: Request) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to search conversations." }, { status: 401 }); }
  const url = new URL(request.url);
  const parsed = querySchema.safeParse(Object.fromEntries(
    ["q", "repositoryId", "sourceSnapshotId", "graphVersion", "limit"].map((key) => [key, url.searchParams.get(key) ?? undefined]),
  ));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: "A bounded query and complete repository context are required." }, { status: 400 });
  if (!authorizeMemoryRequest(session, session.workspaceId, "user")) return NextResponse.json({ outcome: "forbidden", error: "Conversation scope is not authorized." }, { status: 403 });
  try {
    const matches = await searchConversations({
      session,
      context: {
        repositoryId: parsed.data.repositoryId,
        workspaceId: session.workspaceId,
        sourceSnapshotId: parsed.data.sourceSnapshotId,
        graphVersion: parsed.data.graphVersion,
      },
      query: parsed.data.q,
      limit: parsed.data.limit,
    });
    return NextResponse.json({ outcome: matches.length ? "success" : "empty", matches }, { headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Conversation search is unavailable." }, { status: 502 });
  }
}
