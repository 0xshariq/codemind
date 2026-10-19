import type { OperationState } from "./state";

export type OperationEvent = OperationState & { timestamp: string };

export function event(state: OperationState): OperationEvent {
  const terminal = state.status === "completed" || state.status === "partial" || state.status === "failed";
  const lifecycle = state.lifecycle ?? (state.status === "failed" ? "failed" : terminal ? "completed" : state.status === "idle" ? "pending" : "running");
  const outcome = state.outcome ?? (state.status === "completed" ? "success" : state.status === "partial" ? "partial" : state.status === "failed" ? "unavailable" : "unknown");
  const completeness = state.completeness ?? (state.status === "completed" ? "complete" : state.status === "partial" ? "partial" : "unknown");
  return {
    ...state,
    lifecycle,
    outcome,
    completeness,
    ...(terminal && !state.resultReference ? { resultReference: state.operationId } : {}),
    timestamp: new Date().toISOString(),
  };
}
