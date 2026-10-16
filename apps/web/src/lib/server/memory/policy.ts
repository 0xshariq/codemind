import "server-only";

export const MEMORY_SCOPES = ["user", "repository", "team"] as const;
export const MEMORY_KINDS = ["semantic", "procedure"] as const;

export function canReadMemory(input: {
  scope: string;
  requestedScope: string;
  workspaceId: string;
}) {
  return MEMORY_SCOPES.includes(input.scope as (typeof MEMORY_SCOPES)[number])
    && input.scope === input.requestedScope
    && input.workspaceId.trim().length > 0;
}