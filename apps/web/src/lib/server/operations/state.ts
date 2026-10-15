export type OperationStatus = "idle" | "planning" | "coie" | "tie" | "chie" | "reasoning" | "completed" | "partial" | "failed";

export type OperationState = {
  operationId: string;
  status: OperationStatus;
  sequence: number;
  message?: string;
};