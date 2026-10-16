import "server-only";
import type { AuthenticatedSession } from "./context";

export function authorizeMemoryRequest(
  session: AuthenticatedSession,
  requestedWorkspaceId: string,
  requestedScope: string,
) {
  return session.workspaceId === requestedWorkspaceId
    && session.allowedScopes.includes(requestedScope);
}