import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { validateCanonicalResult } from "@/lib/server/results/validate";
import { createRegisteredToolExecutor } from "@/lib/server/tools/executor";
import { toolInputSchema } from "@/lib/server/tools/schemas";
import { getRepository } from "@/lib/server/repositories/service";

export async function POST(request: Request) {
  let session;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to run an analysis." }, { status: 401 });
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ outcome: "invalid", error: "The request body must be valid JSON." }, { status: 400 });
  }

  const parsed = toolInputSchema.safeParse(body);
  if (!parsed.success || !parsed.data.symbolId) {
    return NextResponse.json({ outcome: "invalid", error: "A symbol and complete repository context are required." }, { status: 400 });
  }

  const requestedContext = { ...parsed.data.context, workspaceId: session.workspaceId };
  if (!authorizeMemoryRequest(session, requestedContext.workspaceId, "repository")) {
    return NextResponse.json({ outcome: "unauthorized", error: "You do not have access to this repository scope." }, { status: 403 });
  }

  let repository;
  try { repository = await getRepository(session.workspaceId, requestedContext.repositoryId); }
  catch { return NextResponse.json({ outcome: "unavailable", error: "Repository state is unavailable." }, { status: 502 }); }
  if (!repository) return NextResponse.json({ outcome: "not_found", error: "Repository was not found." }, { status: 404 });
  if (repository.sourceSnapshotId !== requestedContext.sourceSnapshotId || repository.graphVersion !== requestedContext.graphVersion) {
    return NextResponse.json({ outcome: "stale", error: "The selected repository context is stale." }, { status: 409 });
  }
  const context = {
    repositoryId: repository.repositoryId,
    workspaceId: session.workspaceId,
    sourceSnapshotId: repository.sourceSnapshotId!,
    graphVersion: repository.graphVersion!,
  };

  try {
    const result = await createRegisteredToolExecutor()("analyze_change", {
      ...parsed.data,
      context,
    });
    validateCanonicalResult(result);
    const canonical = result as Record<string, unknown>;
    const resultContext = canonical.context as Record<string, unknown> | undefined;
    if (!resultContext || ["repositoryId", "workspaceId", "sourceSnapshotId", "graphVersion"].some(
      (key) => resultContext[key] !== context[key as keyof typeof context],
    )) {
      return NextResponse.json({ outcome: "invalid", error: "The engine returned a result for a different repository context." }, { status: 502 });
    }
    const outcome = canonical.outcome;
    const completeness = canonical.completeness;
    const freshness = canonical.freshness;
    if (!(["success", "empty", "partial", "unsupported", "unavailable", "failed"].includes(String(outcome))
      && ["complete", "partial", "incomplete", "truncated", "unknown"].includes(String(completeness))
      && ["current", "stale", "unknown"].includes(String(freshness)))) {
      return NextResponse.json({ outcome: "invalid", error: "The engine returned malformed result metadata." }, { status: 502 });
    }
    return NextResponse.json({
      outcome,
      completeness,
      freshness,
      context: resultContext,
      data: canonical.data,
      evidence: Array.isArray(canonical.evidence) ? canonical.evidence.slice(0, 128) : [],
      diagnostics: Array.isArray(canonical.diagnostics) ? canonical.diagnostics.slice(0, 128) : [],
      limitations: Array.isArray(canonical.limitations) ? canonical.limitations.slice(0, 64) : [],
    });
  } catch {
    return NextResponse.json(
      { outcome: "unavailable", error: "The analysis service could not return a valid canonical result." },
      { status: 502 },
    );
  }
}
