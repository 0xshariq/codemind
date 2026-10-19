import "server-only";

import path from "node:path";
import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import { getRustClientConfig } from "./client";

export type RustProgressEvent = {
  eventId: string;
  sequence: number;
  engine: string;
  stage: string;
  stageState: string;
  timestamp: string;
  correlation: Record<string, unknown> | null;
  context: Record<string, unknown> | null;
  message: string;
};

type ProgressClient = grpc.Client & {
  streamProgress(
    request: Record<string, unknown>,
    metadata?: grpc.Metadata,
    options?: grpc.CallOptions,
  ): grpc.ClientReadableStream<Record<string, unknown>>;
};

function normalizeEvent(value: Record<string, unknown>): RustProgressEvent {
  const sequenceValue = value.sequence;
  const sequence = typeof sequenceValue === "number" ? sequenceValue : Number(sequenceValue);
  if (!Number.isSafeInteger(sequence) || sequence < 0) {
    throw new Error("Rust progress event has an invalid sequence");
  }
  const message = typeof value.message === "string" ? value.message : "";
  const rawStageState = typeof value.stageState === "string" ? value.stageState.toLowerCase() : "";
  const stageState = ["pending", "running", "completed", "skipped", "failed"].includes(rawStageState)
    ? rawStageState
    : /\bfailed\b/i.test(message) ? "failed"
      : /\bcompleted\b/i.test(message) ? "completed"
        : /\bstarted\b/i.test(message) ? "running" : "unknown";
  return {
    eventId: typeof value.eventId === "string" ? value.eventId : "",
    sequence,
    engine: typeof value.engine === "string" ? value.engine : "unknown",
    stage: typeof value.stage === "string" ? value.stage : "unknown",
    stageState,
    timestamp: typeof value.timestamp === "string" ? value.timestamp : "",
    correlation: value.correlation && typeof value.correlation === "object" ? value.correlation as Record<string, unknown> : null,
    context: value.context && typeof value.context === "object" ? value.context as Record<string, unknown> : null,
    message,
  };
}

export async function* streamRustProgress(input: {
  operationId: string;
  afterSequence: number;
  limit?: number;
  context: {
    repositoryId: string;
    workspaceId: string;
    sourceSnapshotId: string;
    graphVersion: string;
  };
  signal?: AbortSignal;
}): AsyncGenerator<RustProgressEvent> {
  if (!input.operationId.trim() || !Number.isSafeInteger(input.afterSequence) || input.afterSequence < 0) {
    throw new Error("A valid operation ID and sequence are required");
  }
  const config = getRustClientConfig();
  const packageDefinition = await protoLoader.load(
    path.resolve(process.cwd(), "../../proto/v1/operation.proto"),
    { includeDirs: [path.resolve(process.cwd(), "../../proto")] },
  );
  const loaded = grpc.loadPackageDefinition(packageDefinition) as unknown as {
    codemind: { v1: { OperationService: new (endpoint: string, credentials: grpc.ChannelCredentials) => ProgressClient } };
  };
  const client = new loaded.codemind.v1.OperationService(config.endpoint, grpc.credentials.createInsecure());
  const stream = client.streamProgress({
    operationId: input.operationId,
    afterSequence: input.afterSequence,
    limit: Math.max(1, Math.min(input.limit ?? 100, 1_000)),
    context: {
      repositoryId: input.context.repositoryId,
      workspaceId: input.context.workspaceId,
      sourceSnapshotId: input.context.sourceSnapshotId,
      graphVersion: input.context.graphVersion,
    },
  }, new grpc.Metadata());

  const abort = () => stream.cancel();
  input.signal?.addEventListener("abort", abort, { once: true });
  try {
    for await (const event of stream) {
      if (input.signal?.aborted) return;
      yield normalizeEvent(event);
    }
  } finally {
    input.signal?.removeEventListener("abort", abort);
    stream.cancel();
    client.close();
  }
}
