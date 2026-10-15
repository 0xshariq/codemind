import type { OperationEvent } from "./events";
import type { OperationState } from "./state";
import { event } from "./events";

const operations = new Map<string, OperationEvent[]>();
const results = new Map<string, { answer?: string; error?: string }>();
const TERMINAL_TTL_MS = 5 * 60 * 1000;

function scheduleEviction(operationId: string) {
  setTimeout(() => {
    operations.delete(operationId);
    results.delete(operationId);
  }, TERMINAL_TTL_MS).unref?.();
}

export function publishOperation(state: OperationState): OperationEvent {
  const next = event(state);
  const history = operations.get(state.operationId) ?? [];
  history.push(next);
  operations.set(state.operationId, history);
  if (state.status === "completed" || state.status === "failed") scheduleEviction(state.operationId);
  return next;
}

export function getOperation(operationId: string): OperationEvent[] {
  return [...(operations.get(operationId) ?? [])];
}

export function setOperationResult(operationId: string, result: { answer?: string; error?: string }) {
  results.set(operationId, result);
}

export function getOperationResult(operationId: string) {
  return results.get(operationId);
}