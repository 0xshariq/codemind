import type { OperationState } from "./state";

export type OperationEvent = OperationState & { timestamp: string };

export function event(state: OperationState): OperationEvent {
  return { ...state, timestamp: new Date().toISOString() };
}