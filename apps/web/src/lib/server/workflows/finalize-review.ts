import "server-only";

import { createHash } from "node:crypto";
import { persistReviewContext } from "../rust-client/client";
import type { AuthenticatedSession } from "../auth/context";
import { authorizeRepositoryContext } from "../auth/authorize";
import { persistMemoryCandidate } from "../memory/service";

function snakeCase(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(snakeCase);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value).map(([key, nested]) => [
      key.replace(/[A-Z]/g, (character) => `_${character.toLowerCase()}`),
      snakeCase(nested),
    ]),
  );
}

export type ReviewFinalizationInput = {
  authenticatedSession: AuthenticatedSession;
  contextId: string;
  scope: string;
  context: {
    repositoryId: string;
    workspaceId: string;
    sourceSnapshotId: string;
    graphVersion: string;
  };
  session: Record<string, unknown>;
  turns: Record<string, unknown>[];
  episode: Record<string, unknown>;
  findings: Record<string, unknown>[];
  updatedAt: string;
};

export async function finalizeReview(input: ReviewFinalizationInput) {
  if (input.scope !== "user" || !input.contextId.trim() || input.contextId.length > 1_024
    || input.turns.length > 100 || input.findings.length > 64) {
    throw new Error("review scope, identity, or bounded persistence limits are invalid");
  }
  if (!await authorizeRepositoryContext(input.authenticatedSession, input.context)
    || input.authenticatedSession.workspaceId !== input.context.workspaceId) {
    throw new Error("review context is not authorized or no longer current");
  }

  const payload = snakeCase({
    context_id: input.contextId,
    scope: input.scope,
    session: input.session,
    turns: input.turns,
    episode: input.episode,
    findings: input.findings,
    context: {
      repository_id: input.context.repositoryId,
      workspace_id: input.context.workspaceId,
      source_snapshot_id: input.context.sourceSnapshotId,
      graph_version: input.context.graphVersion,
    },
    updated_at: input.updatedAt,
  });

  const payloadJson = JSON.stringify(payload);
  if (Buffer.byteLength(payloadJson, "utf8") > 16 * 1024) {
    throw new Error("review finalization payload exceeds 16 KiB");
  }

  const persisted = await persistReviewContext({
    contextId: input.contextId,
    scope: input.scope,
    payloadJson,
    context: input.context,
    updatedAt: input.updatedAt,
    }, input.authenticatedSession.userId);
  let candidateCount = 0;
  for (const finding of input.findings) {
    const summary = typeof finding.summary === "string" ? finding.summary.trim().slice(0, 2_000) : "";
    const rawEvidence = Array.isArray(finding.evidence) ? finding.evidence : [];
    const evidence = rawEvidence.flatMap((item) => {
      if (!item || typeof item !== "object") return [];
      const value = item as Record<string, unknown>;
      const path = value.path;
      const startLine = Number(value.startLine ?? value.start_line ?? value.line);
      const endLine = Number(value.endLine ?? value.end_line ?? value.line);
      const snapshotId = value.snapshotId ?? value.snapshot_id;
      if (typeof path !== "string" || typeof snapshotId !== "string" || !Number.isSafeInteger(startLine) || !Number.isSafeInteger(endLine)) return [];
      return [{ path, startLine, endLine, snapshotId }];
    }).slice(0, 128);
    if (!summary || evidence.length === 0) continue;
    const idempotencyKey = createHash("sha256").update(`${input.contextId}\0${summary}\0${JSON.stringify(evidence)}`).digest("hex");
    await persistMemoryCandidate({
      session: input.authenticatedSession,
      candidate: {
        memoryId: `review-${idempotencyKey}`,
        kind: "semantic",
        scope: "user",
        lifecycle: "candidate",
        freshness: "unknown",
        payloadJson: JSON.stringify({ summary, source: "review_finding", contextId: input.contextId }),
        context: input.context,
        evidence,
        idempotencyKey,
        version: 0,
      },
    });
    candidateCount += 1;
  }
  return { review: persisted, memoryCandidatesCreated: candidateCount };
}
