import "server-only";
import { retrieveMemory } from "@/lib/server/rust-client/client";
import { canReadMemory } from "./policy";
import { classifyFreshness } from "./freshness";
import type { AuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryOwner } from "@/lib/server/auth/authorize";

export async function retrieveAuthorizedMemory(input: {
  context: Record<string, string>;
  scope: string;
  kind: string;
  session: AuthenticatedSession;
  ownerId?: string;
}) {
  const ownerId = input.ownerId ?? (input.scope === "repository" ? input.context.repositoryId : input.scope === "user" ? input.session.userId : "");
  if (!authorizeMemoryOwner(input.session, {
    scope: input.scope,
    ownerId,
    workspaceId: input.context.workspaceId,
    repositoryId: input.context.repositoryId,
  })
    || !canReadMemory({
      scope: input.scope,
      requestedScope: input.scope,
      workspaceId: input.session.workspaceId,
      allowedScopes: input.session.allowedScopes,
    })) {
    throw new Error("memory scope is not authorized");
  }
  const requestedContext: Record<string, string> = {
    repositoryId: input.context.repositoryId,
    workspaceId: input.session.workspaceId,
    sourceSnapshotId: input.context.sourceSnapshotId,
    graphVersion: input.context.graphVersion,
  };
  const response = await retrieveMemory({
    context: requestedContext,
    scope: input.scope,
    kind: input.kind,
    ownerId,
    limit: 20,
  }, input.session.userId);
  const memories = Array.isArray(response.memories) ? response.memories.slice(0, 20) : [];
  return memories.map((memory) => {
    const value = memory as Record<string, unknown>;
    const storedContext = (value.context ?? {}) as Record<string, string>;
    return {
      ...value,
      freshness: classifyFreshness(
        storedContext.sourceSnapshotId ?? "",
        requestedContext.sourceSnapshotId,
        String(value.lifecycle ?? ""),
        storedContext.graphVersion ?? "",
        requestedContext.graphVersion,
        storedContext.repositoryId === requestedContext.repositoryId
          && storedContext.workspaceId === requestedContext.workspaceId,
      ),
    };
  });
}
