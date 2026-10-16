import "server-only";

import { persistReviewContext } from "../rust-client/client";

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

  const payload = {
    contextId: input.contextId,
    scope: input.scope,
    session: input.session,
    turns: input.turns,
    episode: input.episode,
    findings: input.findings,
    context: input.context,
    updatedAt: input.updatedAt,
  };

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