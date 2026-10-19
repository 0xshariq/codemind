import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { getRepository } from "@/lib/server/repositories/service";

export async function GET(request: Request, context: { params: Promise<{ repositoryId: string }> }) {
  let session;
  try { session = loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to view repository details." }, { status: 401 }); }
  const { repositoryId } = await context.params;
  if (!repositoryId.trim() || repositoryId.length > 1_024) {
    return NextResponse.json({ outcome: "invalid", error: "A valid repository ID is required." }, { status: 400 });
  }
  if (!authorizeMemoryRequest(session, session.workspaceId, "repository")) {
    return NextResponse.json({ outcome: "forbidden", error: "Repository scope is not authorized." }, { status: 403 });
  }
  try {
    const repository = await getRepository(session.workspaceId, repositoryId);
    if (!repository) return NextResponse.json({ outcome: "not_found", repositoryId }, { status: 404 });
    return NextResponse.json({ outcome: repository.readiness === "partial" ? "partial" : "success", repository, limitations: repository.limitations }, { headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Repository service is unavailable." }, { status: 502 });
  }
}
