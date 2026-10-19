import "server-only";
import type { AuthenticatedSession } from "./context";
import { getRepository } from "../repositories/service";
import { canAccessOperation } from "../operations/service";

export type AuthorizedRepositoryContext = {
  workspaceId: string;
  repositoryId: string;
  sourceSnapshotId: string;
  graphVersion: string;
};

export function authorizeMemoryRequest(
  session: AuthenticatedSession,
  requestedWorkspaceId: string,
  requestedScope: string,
) {
  return session.workspaceId === requestedWorkspaceId
    && session.allowedScopes.includes(requestedScope);
}

export async function authorizeRepositoryContext(
  session: AuthenticatedSession,
  context: AuthorizedRepositoryContext,
): Promise<boolean> {
  if (context.workspaceId !== session.workspaceId || !session.allowedScopes.includes("repository")) return false;
  try {
    const repository = await getRepository(session.workspaceId, context.repositoryId);
    return Boolean(repository
      && repository.sourceSnapshotId === context.sourceSnapshotId
      && repository.graphVersion === context.graphVersion);
  } catch {
    return false;
  }
}

export function authorizeMemoryOwner(
  session: AuthenticatedSession,
  input: { scope: string; ownerId: string; workspaceId: string; repositoryId?: string },
): boolean {
  if (input.workspaceId !== session.workspaceId || !session.allowedScopes.includes(input.scope)) return false;
  if (input.scope === "user") return input.ownerId === session.userId;
  if (input.scope === "repository") return input.repositoryId === input.ownerId;
  if (input.scope === "team") return (session.teamIds ?? []).includes(input.ownerId);
  return false;
}

export function authorizeOperationOwner(session: AuthenticatedSession, operationId: string): boolean {
  return canAccessOperation(operationId, session);
}
