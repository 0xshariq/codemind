import "server-only";
import {
  persistHandoff,
  retrieveHandoff,
  transitionHandoff,
} from "@/lib/server/rust-client/client";
import { handoffSchema, type Handoff } from "./schema";

export async function saveHandoff(input: Handoff) {
  const handoff = handoffSchema.parse(input);
  const payload = {
    handoff_id: handoff.handoffId,
    schema_version: handoff.schemaVersion,
    operation_id: handoff.operationId,
    review_episode_id: handoff.reviewEpisodeId,
    source_stage: handoff.sourceStage,
    target_stage: handoff.targetStage,
    task: handoff.task,
    findings: handoff.findings,
    evidence_references: handoff.evidenceReferences.map((reference) => ({
      path: reference.path,
      start_line: reference.startLine,
      end_line: reference.endLine,
      snapshot_id: reference.snapshotId,
    })),
    context: {
      repository_id: handoff.context.repositoryId,
      workspace_id: handoff.context.workspaceId,
      source_snapshot_id: handoff.context.sourceSnapshotId,
      graph_version: handoff.context.graphVersion,
    },
    open_questions: handoff.openQuestions,
    status: handoff.status,
    idempotency_key: handoff.idempotencyKey,
    version: handoff.version,
    created_at: handoff.createdAt,
    updated_at: handoff.updatedAt,
  };
  return persistHandoff({
    handoffId: handoff.handoffId,
    status: handoff.status,
    payloadJson: JSON.stringify(payload),
    context: handoff.context,
  }, handoff.idempotencyKey);
}

export async function loadHandoff(
  handoffId: string,
  context: Handoff["context"],
) {
  return retrieveHandoff(handoffId, context);
}

export async function updateHandoff(
  handoffId: string,
  status: Handoff["status"],
  context: Handoff["context"],
) {
  return transitionHandoff(handoffId, status, context);
}