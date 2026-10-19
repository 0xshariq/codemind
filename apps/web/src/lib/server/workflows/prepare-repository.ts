import "server-only";
import type { AuthenticatedSession } from "../auth/context";
import { authorizeMemoryRequest } from "../auth/authorize";
import { prepareConfiguredRepository } from "../repositories/service";

export async function prepareRepository(input: {
  session: AuthenticatedSession;
  sourceType: "local" | "example" | "github";
  source: string;
  name?: string;
}) {
  if (!authorizeMemoryRequest(input.session, input.session.workspaceId, "repository")) {
    return { outcome: "invalid" as const, readiness: "unavailable" as const, message: "Repository scope is not authorized." };
  }
  const result = await prepareConfiguredRepository({
    workspaceId: input.session.workspaceId,
    sourceType: input.sourceType,
    source: input.source,
    name: input.name,
  });
  const resolved = result.outcome === "completed" || result.outcome === "partial";
  return {
    ...result,
    readiness: result.outcome === "completed" ? "snapshot_resolved" as const
      : result.outcome === "partial" ? "partial" as const
        : result.outcome === "unsupported" ? "unsupported" as const
          : result.outcome === "failed" ? "failed" as const : "unavailable" as const,
    graphReadiness: resolved ? "pending_investigation" as const : "not_ready" as const,
    limitations: [
      ...(Array.isArray(result.limitations) ? result.limitations : []),
      ...(resolved ? ["Graph construction and TIE discovery run when an investigation selects a symbol; the Rust proto has no standalone repository-preparation operation."] : []),
    ],
  };
}
