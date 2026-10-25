import type { RustProgressEvent } from "@/lib/server/rust-client/progress-stream";
import type { OperationStatus, OperationState } from "./state";

function mapStage(stage: string): OperationStatus {
  const value = stage.trim().toLowerCase();
  if (value.includes("tie") || value.includes("test intelligence")) return "tie";
  if (value.includes("chie") || value.includes("change intelligence")) return "chie";
  if (value.includes("coie") || value.includes("code intelligence")) return "coie";
  if (value.includes("reason") || value.includes("llm") || value.includes("response")) return "reasoning";
  return "planning";
}

export function rustProgressState(operationId: string, sequence: number, progress: RustProgressEvent): OperationState {
  const status = progress.stageState === "failed" ? "failed" : mapStage(progress.stage);
  return {
    operationId,
    status,
    sequence,
    message: progress.message || `${progress.stage} progress`,
    engine: progress.engine || "rust-engine",
    stage: progress.stage || "unknown",
    stageState: progress.stageState,
    progressSequence: progress.sequence,
    progressEventId: progress.eventId || undefined,
    lifecycle: progress.stageState === "failed" ? "failed" : progress.stageState === "completed" ? "completed" : "running",
    outcome: progress.stageState === "failed" ? "unavailable" : "unknown",
    completeness: "unknown",
  };
}
