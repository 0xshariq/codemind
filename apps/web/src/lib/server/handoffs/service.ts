import "server-only";
import {
  persistHandoff,
  retrieveHandoff,
  transitionHandoff,
} from "@/lib/server/rust-client/client";
import { handoffSchema, type Handoff } from "./schema";

export async function saveHandoff(input: Handoff) {
  const handoff = handoffSchema.parse(input);
  return persistHandoff({
    handoffId: handoff.handoffId,
    status: handoff.status,
    payloadJson: JSON.stringify(handoff),
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