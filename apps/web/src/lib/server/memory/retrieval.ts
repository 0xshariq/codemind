import "server-only";
import { retrieveMemory } from "@/lib/server/rust-client/client";
import { canReadMemory } from "./policy";
import { classifyFreshness } from "./freshness";
import type { AuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";

export async function retrieveAuthorizedMemory(input: {
  context: Record<string, string>;
  scope: string;
  kind: string;
  session: AuthenticatedSession;
}) {
  if (!authorizeMemoryRequest(input.session, input.context.workspaceId, input.scope)
    || !canReadMemory({
      scope: input.scope,
      requestedScope: input.scope,
      workspaceId: input.session.workspaceId,
      allowedScopes: input.session.allowedScopes,
    })) {
    throw new Error("memory scope is not authorized");
  }
  const context = { ...input.context, workspaceId: input.session.workspaceId };
  const response = await retrieveMemory({ ...input, context });
  const memories = Array.isArray(response.memories) ? response.memories : [];
  return memories.map((memory) => {
    const value = memory as Record<string, unknown>;
    const context = (value.context ?? {}) as Record<string, string>;
    return {
      ...value,
      freshness: classifyFreshness(
        context.sourceSnapshotId ?? "",
        context.sourceSnapshotId,
        String(value.lifecycle ?? ""),
      ),
    };
  });
}