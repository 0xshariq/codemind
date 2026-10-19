import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { validateCanonicalResult } from "@/lib/server/results/validate";
import { projectForUi } from "@/lib/server/results/ui-projection";
import { createRegisteredToolExecutor } from "@/lib/server/tools/executor";
import { toolInputSchema } from "@/lib/server/tools/schemas";
import { getRepository } from "@/lib/server/repositories/service";

export async function POST(request: Request) {
  let session;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to query repository relationships." }, { status: 401 });
  }

  let body: unknown = null;
  try { body = await request.json(); } catch { body = null; }
  const parsed = toolInputSchema.safeParse(body);
  if (!parsed.success || !parsed.data.symbolId) {
    return NextResponse.json({ outcome: "invalid", error: "symbolId and complete context are required" }, { status: 400 });
  }
  const requestedContext = { ...parsed.data.context, workspaceId: session.workspaceId };
  if (!authorizeMemoryRequest(session, requestedContext.workspaceId, "repository")) {
    return NextResponse.json({ outcome: "forbidden", error: "You do not have access to this repository scope." }, { status: 403 });
  }
  let repository;
  try { repository = await getRepository(session.workspaceId, requestedContext.repositoryId); }
  catch { return NextResponse.json({ outcome: "unavailable", error: "Repository state is unavailable." }, { status: 502 }); }
  if (!repository) return NextResponse.json({ outcome: "not_found", error: "Repository was not found." }, { status: 404 });
  if (!repository.sourceSnapshotId || !repository.graphVersion
    || repository.sourceSnapshotId !== requestedContext.sourceSnapshotId
    || repository.graphVersion !== requestedContext.graphVersion) {
    return NextResponse.json({ outcome: "stale", error: "The selected repository context is stale." }, { status: 409 });
  }
  const context = {
    repositoryId: repository.repositoryId,
    workspaceId: session.workspaceId,
    sourceSnapshotId: repository.sourceSnapshotId,
    graphVersion: repository.graphVersion,
  };

  try {
    const result = await createRegisteredToolExecutor()("query_code_relationships", {
      ...parsed.data,
      context,
    });
    validateCanonicalResult(result);
    const canonical = result as Record<string, unknown>;
    if (JSON.stringify(canonical.context) !== JSON.stringify(context)) {
      return NextResponse.json({ outcome: "invalid", error: "The engine returned a result for a different repository context." }, { status: 502 });
    }
    return NextResponse.json(projectForUi(result));
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "The relationship service could not return a valid result." }, { status: 502 });
  }
}
