export type OperationStatus = "idle" | "planning" | "coie" | "tie" | "chie" | "reasoning" | "completed" | "partial" | "failed";
export type OperationLifecycle = "pending" | "running" | "completed" | "failed";
export type OperationOutcome = "unknown" | "success" | "partial" | "invalid" | "unavailable" | "unsupported";
export type OperationCompleteness = "unknown" | "complete" | "partial" | "truncated";

export type OperationState = {
  operationId: string;
  status: OperationStatus;
  sequence: number;
  message?: string;
  correlationId?: string;
  engine?: string;
  stage?: string;
  stageState?: string;
  progressSequence?: number;
  progressEventId?: string;
  lifecycle?: OperationLifecycle;
  outcome?: OperationOutcome;
  completeness?: OperationCompleteness;
  resultReference?: string;
  limitation?: string;
  errorCategory?: "invalid" | "unauthorized" | "unavailable" | "conflict" | "internal";
};
