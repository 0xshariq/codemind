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
  const history = operations.get(state.operationId) ?? [];
  const last = history.at(-1);
  if (last && (state.sequence <= last.sequence || isTerminal(last.status))) {
    return last;
  }
  const next = event(state);
  history.push(next);
  operations.set(state.operationId, history);
  if (isTerminal(state.status)) scheduleEviction(state.operationId);
  return next;
}

function isTerminal(status: OperationState["status"]) {
  return status === "completed" || status === "failed";
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