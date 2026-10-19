import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { canAccessOperation, getOperation, getOperationResult } from "@/lib/server/operations/service";
import { validateCanonicalResult } from "@/lib/server/results/validate";

export async function GET(request: Request, { params }: { params: Promise<{ analysisId: string }> }) {
  let session;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to view this analysis." }, { status: 401 });
  }

  const { analysisId } = await params;
  if (!analysisId.trim() || analysisId.length > 1_024) {
    return NextResponse.json({ outcome: "invalid", error: "A valid analysis ID is required." }, { status: 400 });
  }
  if (!canAccessOperation(analysisId, session)) {
    return NextResponse.json({ outcome: "not_found", error: "Analysis was not found." }, { status: 404 });
  }
  const events = getOperation(analysisId);
  if (!events.length) {
    return NextResponse.json({ outcome: "not_found", error: "Analysis state has expired." }, { status: 404 });
  }

  const latest = events.at(-1)!;
  const stored = getOperationResult(analysisId);
  const terminal = latest.status === "completed" || latest.status === "failed";
  let result: Record<string, unknown> | null = null;
  if (stored?.canonical) {
    try {
      validateCanonicalResult(stored.canonical);
      const canonical = stored.canonical as Record<string, unknown>;
      const context = canonical.context as Record<string, unknown> | undefined;
      if (!context || (context.workspaceId ?? context.workspace_id) !== session.workspaceId) throw new Error("result context is unauthorized");
      result = {
        outcome: canonical.outcome,
        completeness: canonical.completeness,
        freshness: canonical.freshness,
        context: {
          repository_id: context.repositoryId ?? context.repository_id,
          workspace_id: context.workspaceId ?? context.workspace_id,
          source_snapshot_id: context.sourceSnapshotId ?? context.source_snapshot_id,
          graph_version: context.graphVersion ?? context.graph_version,
        },
        data: canonical.data,
        evidence: Array.isArray(canonical.evidence) ? canonical.evidence : [],
        diagnostics: Array.isArray(canonical.diagnostics) ? canonical.diagnostics : [],
        limitations: Array.isArray(canonical.limitations) ? canonical.limitations : [],
      };
    } catch {
      result = null;
    }
  }
  return NextResponse.json({
    analysisId,
    lifecycle: latest.status === "failed" ? "failed" : latest.status === "completed" ? "completed" : "running",
    result,
    events: events.map((item) => ({ sequence: item.sequence, status: item.status, message: item.message, timestamp: item.timestamp })),
    error: latest.status === "failed" ? { code: "analysis_failed", message: stored?.error ?? "Analysis failed." } : null,
    limitations: result ? [] : ["A canonical analysis result is not available for this operation."],
  }, { headers: { "cache-control": "no-store" } });
}
