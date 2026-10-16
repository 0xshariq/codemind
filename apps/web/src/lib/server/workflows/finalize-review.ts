import "server-only";

import { persistReviewContext } from "../rust-client/client";

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
  if (input.turns.length > 100 || input.findings.length > 64) {
    throw new Error("review finalization exceeds bounded persistence limits");
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

  if (JSON.stringify(payload).length > 16 * 1024) {
    throw new Error("review finalization payload exceeds 16 KiB");
  }

  return persistReviewContext({
    contextId: input.contextId,
    scope: input.scope,
    payloadJson: JSON.stringify(payload),
    context: input.context,
    updatedAt: input.updatedAt,
  });
}