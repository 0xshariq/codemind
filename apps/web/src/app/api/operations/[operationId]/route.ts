import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { canAccessOperation, getOperation, getOperationResult } from "@/lib/server/operations/service";
import { validateCanonicalResult } from "@/lib/server/results/validate";

export async function GET(request: Request, { params }: { params: Promise<{ operationId: string }> }) {
  let session;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to view this operation." }, { status: 401 });
  }
  const { operationId } = await params;
  if (!operationId.trim() || operationId.length > 1_024) {
    return NextResponse.json({ outcome: "invalid", error: "A valid operation ID is required." }, { status: 400 });
  }
  if (!canAccessOperation(operationId, session)) {
    return NextResponse.json({ outcome: "not_found", error: "Operation was not found." }, { status: 404 });
  }

  const events = getOperation(operationId);
  const result = getOperationResult(operationId);
  if (events.length === 0) {
    return NextResponse.json({ outcome: "not_found", error: "Operation state has expired." }, { status: 404 });
  }
  const status = events.at(-1)?.status ?? "unknown";
  const lifecycle = status === "completed" || status === "partial" || status === "failed" ? "terminal" : "running";
  let canonicalResult: unknown = null;
  if (result?.canonical) {
    try {
      validateCanonicalResult(result.canonical);
      const context = (result.canonical as Record<string, unknown>).context as Record<string, unknown> | undefined;
      if ((context?.workspaceId ?? context?.workspace_id) === session.workspaceId) canonicalResult = result.canonical;
    } catch { /* Invalid canonical data is never returned. */ }
  }
  return NextResponse.json({
    operationId,
    lifecycle,
    outcome: status === "failed" ? "failed" : status === "partial" ? "partial" : status === "completed" ? "success" : "unavailable",
    completeness: status === "partial" ? "partial" : status === "completed" ? "unknown" : "incomplete",
    events,
    result: canonicalResult,
    explanation: result?.answer ?? null,
    error: result?.error ? { code: "operation_failed", message: result.error } : null,
    limitations: canonicalResult ? ["Operation state is held by the web process and may expire."] : ["Operation state is held by the web process; a canonical result reference is unavailable."],
  }, { headers: { "cache-control": "no-store" } });
}
