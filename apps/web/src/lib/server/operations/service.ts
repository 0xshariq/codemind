import type { OperationEvent } from "./events";
import type { OperationState } from "./state";
import { event } from "./events";

const operations = new Map<string, OperationEvent[]>();
const results = new Map<string, { answer?: string; error?: string }>();

export function publishOperation(state: OperationState): OperationEvent {
  const next = event(state);
  const history = operations.get(state.operationId) ?? [];
  history.push(next);
  operations.set(state.operationId, history);
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