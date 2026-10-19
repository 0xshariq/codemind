import type { RustProgressEvent } from "@/lib/server/rust-client/progress-stream";
import type { OperationStatus, OperationState } from "./state";

export function rustProgressState(
  operationId: string,
  sequence: number,
  progress: RustProgressEvent,
): OperationState {
  const stage = progress.stage.trim().toLowerCase();
  const status: OperationStatus = stage.includes("tie") ? "tie"
    : stage.includes("chie") ? "chie"
      : stage.includes("coie") ? "coie" : "planning";
  return {
    operationId,
    status,
    sequence,
    message: progress.message || `${progress.stage} progress`,
    engine: progress.engine,
    stage: progress.stage,
    stageState: progress.stageState,
    progressSequence: progress.sequence,
    progressEventId: progress.eventId,
  };
}
