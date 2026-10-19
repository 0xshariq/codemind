import "server-only";

export const MEMORY_SCOPES = ["user", "repository", "team"] as const;
export const MEMORY_KINDS = ["semantic", "procedure"] as const;

export function canWriteMemory(input: {
  scope: string;
  lifecycle: string;
  workspaceId: string;
  ownerId: string;
  evidenceCount: number;
  allowedScopes: readonly string[];
}) {
  return MEMORY_SCOPES.includes(input.scope as (typeof MEMORY_SCOPES)[number])
    && input.lifecycle === "candidate"
    && input.workspaceId.trim().length > 0
    && input.ownerId.trim().length > 0
    && input.evidenceCount <= 128
    && input.allowedScopes.includes(input.scope);
}

export function canPromoteMemory(input: {
  lifecycle: string;
  evidenceCount: number;
  evidenceValidated: boolean;
  snapshotMatches: boolean;
  graphVersionMatches: boolean;
}) {
  return input.lifecycle === "candidate"
    && input.evidenceCount > 0
    && input.evidenceValidated
    && input.snapshotMatches
    && input.graphVersionMatches;
}

export function canReadMemory(input: {
  scope: string;
  requestedScope: string;
  workspaceId: string;
  allowedScopes: readonly string[];
}) {
  return MEMORY_SCOPES.includes(input.scope as (typeof MEMORY_SCOPES)[number])
    && input.scope === input.requestedScope
    && input.workspaceId.trim().length > 0
    && input.allowedScopes.includes(input.requestedScope);
}
