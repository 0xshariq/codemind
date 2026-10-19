import "server-only";
import {
  persistHandoff,
  retrieveHandoff,
  transitionHandoff,
} from "@/lib/server/rust-client/client";
import { canTransitionHandoff, handoffSchema, type Handoff } from "./schema";
import type { AuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeOperationOwner, authorizeRepositoryContext } from "@/lib/server/auth/authorize";

async function authorizeHandoff(session: AuthenticatedSession, handoff: Handoff["context"], operationId: string) {
  if (!await authorizeRepositoryContext(session, handoff) || !authorizeOperationOwner(session, operationId)) {
    throw new Error("handoff context or operation is not authorized");
  }
}

function camelize(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(camelize);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.entries(value).map(([key, child]) => [
    key.replace(/_([a-z])/g, (_, character: string) => character.toUpperCase()), camelize(child),
  ]));
}

export async function saveHandoff(input: Handoff, session: AuthenticatedSession) {
  const handoff = handoffSchema.parse(input);
  await authorizeHandoff(session, handoff.context, handoff.operationId);
  if (handoff.status !== "pending" || handoff.version !== 0) {
    throw new Error("new handoffs must begin pending at version zero");
  }
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
  }, handoff.idempotencyKey, session.userId);
}

export async function loadHandoff(
  handoffId: string,
  context: Handoff["context"],
  session: AuthenticatedSession,
  operationId: string,
) {
  await authorizeHandoff(session, context, operationId);
  return retrieveHandoff(handoffId, context, session.userId);
}

export async function updateHandoff(
  handoffId: string,
  status: Handoff["status"],
  context: Handoff["context"],
  session: AuthenticatedSession,
  operationId: string,
) {
  await authorizeHandoff(session, context, operationId);
  const current = await retrieveHandoff(handoffId, context, session.userId);
  const storedHandoff = current.handoff && typeof current.handoff === "object" ? current.handoff as Record<string, unknown> : null;
  const raw = storedHandoff && typeof (storedHandoff.payloadJson ?? storedHandoff.payload_json) === "string"
    ? String(storedHandoff.payloadJson ?? storedHandoff.payload_json) : null;
  if (!raw) throw new Error("handoff record is unavailable");
  const stored = handoffSchema.parse(camelize(JSON.parse(raw)));
  if (stored.operationId !== operationId || stored.context.repositoryId !== context.repositoryId
    || stored.context.sourceSnapshotId !== context.sourceSnapshotId || stored.context.graphVersion !== context.graphVersion) {
    throw new Error("handoff record context does not match the authorized operation");
  }
  if (!canTransitionHandoff(stored.status, status)) throw new Error("handoff lifecycle transition is invalid");
  return transitionHandoff(handoffId, status, context, session.userId);
}
