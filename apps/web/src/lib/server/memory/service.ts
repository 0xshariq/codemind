import "server-only";
import { z } from "zod";
import { authorizeMemoryOwner, authorizeRepositoryContext } from "@/lib/server/auth/authorize";
import type { AuthenticatedSession } from "@/lib/server/auth/context";
import { persistMemory } from "@/lib/server/rust-client/client";
import { canPromoteMemory, canWriteMemory } from "./policy";

const candidateSchema = z.object({
  memoryId: z.string().trim().min(1).max(1_024),
  kind: z.enum(["semantic", "procedure"]),
  scope: z.enum(["user", "repository", "team"]),
  lifecycle: z.literal("candidate"),
  freshness: z.enum(["fresh", "stale", "unknown"]),
  payloadJson: z.string().max(16_384).refine((value) => {
    try { const parsed: unknown = JSON.parse(value); return !!parsed && typeof parsed === "object" && !Array.isArray(parsed); }
    catch { return false; }
  }),
  context: z.object({
    repositoryId: z.string().trim().min(1).max(1_024),
    workspaceId: z.string().trim().min(1).max(1_024),
    sourceSnapshotId: z.string().trim().min(1).max(1_024),
    graphVersion: z.string().trim().min(1).max(1_024),
  }).strict(),
  evidence: z.array(z.object({
    path: z.string().trim().min(1).max(2_048).refine((path) => !path.startsWith("/") && !path.split(/[\\/]/).includes("..")),
    startLine: z.number().int().positive(),
    endLine: z.number().int().positive(),
    snapshotId: z.string().trim().min(1).max(1_024),
  }).strict()).max(128),
  idempotencyKey: z.string().trim().min(1).max(256),
  version: z.number().int().nonnegative(),
}).strict();

export async function persistMemoryCandidate(input: {
  session: AuthenticatedSession;
  candidate: unknown;
  teamId?: string;
}) {
  const candidate = candidateSchema.parse(input.candidate);
  const ownerId = candidate.scope === "user" ? input.session.userId
    : candidate.scope === "repository" ? candidate.context.repositoryId : input.teamId ?? "";
  if (!canWriteMemory({
    scope: candidate.scope,
    lifecycle: candidate.lifecycle,
    workspaceId: candidate.context.workspaceId,
    ownerId,
    evidenceCount: candidate.evidence.length,
    allowedScopes: input.session.allowedScopes,
  }) || !authorizeMemoryOwner(input.session, {
    scope: candidate.scope,
    ownerId,
    workspaceId: candidate.context.workspaceId,
    repositoryId: candidate.context.repositoryId,
  }) || !await authorizeRepositoryContext(input.session, candidate.context)) {
    throw new Error("memory candidate is not authorized for this repository context");
  }
  if (candidate.evidence.some((item) => item.snapshotId !== candidate.context.sourceSnapshotId || item.endLine < item.startLine)) {
    throw new Error("memory candidate evidence does not match its source snapshot");
  }
  const now = new Date().toISOString();
  return persistMemory({
    ...candidate,
    ownerId,
    createdAt: now,
    updatedAt: now,
    freshness: "unknown",
  }, input.session.userId);
}

export function canPromoteMemoryCandidate(input: Parameters<typeof canPromoteMemory>[0]) {
  return canPromoteMemory(input);
}
