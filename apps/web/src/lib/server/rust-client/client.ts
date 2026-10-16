import "server-only";
import path from "node:path";
import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";

export type RustClientConfig = {
  endpoint: string;
  timeoutMs: number;
};

export function getRustClientConfig(
  env: NodeJS.ProcessEnv = process.env,
): RustClientConfig {
  const endpoint = env.CODEMIND_ENGINE_ENDPOINT ?? "127.0.0.1:50051";
  const timeoutMs = Number(env.CODEMIND_ENGINE_TIMEOUT_MS ?? "5000");

  if (!Number.isInteger(timeoutMs) || timeoutMs <= 0) {
    throw new Error("CODEMIND_ENGINE_TIMEOUT_MS must be a positive integer");
  }

  return { endpoint, timeoutMs };
}

export async function querySymbolRelationships(input: {
  symbolId: string;
  limit: number;
  context: {
    repositoryId: string;
    workspaceId: string;
    sourceSnapshotId: string;
    graphVersion: string;
  };
}) {
  const config = getRustClientConfig();
  const packageDefinition = await protoLoader.load(
    path.resolve(process.cwd(), "../../proto/v1/intelligence.proto"),
    { includeDirs: [path.resolve(process.cwd(), "../../proto")] },
  );
  const loaded = grpc.loadPackageDefinition(packageDefinition) as unknown as {
    codemind: {
      v1: {
        IntelligenceService: new (
          address: string,
          credentials: grpc.ChannelCredentials,
        ) => grpc.Client & {
          querySymbolRelationships(
            request: Record<string, unknown>,
            callback: (error: grpc.ServiceError | null, response: Record<string, unknown>) => void,
          ): grpc.ClientUnaryCall;
        };
      };
    };
  };
  const client = new loaded.codemind.v1.IntelligenceService(
    config.endpoint,
    grpc.credentials.createInsecure(),
  );
  return new Promise<Record<string, unknown>>((resolve, reject) => {
    client.querySymbolRelationships(
      {
        symbolId: input.symbolId,
        limit: input.limit,
        context: {
          repositoryId: input.context.repositoryId,
          workspaceId: input.context.workspaceId,
          sourceSnapshotId: input.context.sourceSnapshotId,
          graphVersion: input.context.graphVersion,
        },
      },
      (error, response) => {
        client.close();
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export async function analyzeChange(input: {
  symbolId: string;
  maxDepth: number;
  context: {
    repositoryId: string;
    workspaceId: string;
    sourceSnapshotId: string;
    graphVersion: string;
  };
}) {
  const config = getRustClientConfig();
  const packageDefinition = await protoLoader.load(
    path.resolve(process.cwd(), "../../proto/v1/intelligence.proto"),
    { includeDirs: [path.resolve(process.cwd(), "../../proto")] },
  );
  const loaded = grpc.loadPackageDefinition(packageDefinition) as unknown as {
    codemind: { v1: { IntelligenceService: new (
      address: string, credentials: grpc.ChannelCredentials
    ) => grpc.Client & { analyzeChange(
      request: Record<string, unknown>,
      callback: (error: grpc.ServiceError | null, response: Record<string, unknown>) => void,
    ): grpc.ClientUnaryCall } } };
  };
  const client = new loaded.codemind.v1.IntelligenceService(config.endpoint, grpc.credentials.createInsecure());
  return new Promise<Record<string, unknown>>((resolve, reject) => {
    client.analyzeChange({
      symbolId: input.symbolId,
      maxDepth: input.maxDepth,
      context: input.context,
    }, (error, response) => {
      client.close();
      if (error) reject(error);
      else resolve(response);
    });
  });
}

type MemoryRpc = {
  [method: string]: (
    request: Record<string, unknown>,
    options: grpc.CallOptions,
    callback: (error: grpc.ServiceError | null, response: Record<string, unknown>) => void,
  ) => grpc.ClientUnaryCall;
};

async function memoryClient() {
  const config = getRustClientConfig();
  const packageDefinition = await protoLoader.load(
    path.resolve(process.cwd(), "../../proto/v1/memory.proto"),
    { includeDirs: [path.resolve(process.cwd(), "../../proto")] },
  );
  const loaded = grpc.loadPackageDefinition(packageDefinition) as unknown as {
    codemind: { v1: { MemoryService: new (address: string, credentials: grpc.ChannelCredentials) => grpc.Client & MemoryRpc } };
  };
  return {
    client: new loaded.codemind.v1.MemoryService(config.endpoint, grpc.credentials.createInsecure()),
    timeoutMs: config.timeoutMs,
  };
}

function callMemory(
  method: keyof MemoryRpc,
  request: Record<string, unknown>,
): Promise<Record<string, unknown>> {
  return memoryClient().then(({ client, timeoutMs }) => new Promise((resolve, reject) => {
    client[method](request, { deadline: Date.now() + timeoutMs }, (error, response) => {
      client.close();
      if (error) reject(error);
      else resolve(response);
    });
  }));
}

export function persistMemory(memory: Record<string, unknown>) {
  return callMemory("persistMemory", { memory });
}

export function retrieveMemory(input: {
  context: Record<string, string>;
  scope: string;
  kind: string;
  limit?: number;
}) {
  return callMemory("retrieveMemory", { ...input, limit: input.limit ?? 20 });
}

export function persistHandoff(handoff: Record<string, unknown>, idempotencyKey: string) {
  return callMemory("persistHandoff", { handoff, idempotencyKey });
}

export function retrieveHandoff(handoffId: string, context: Record<string, string>) {
  return callMemory("retrieveHandoff", { handoffId, context });
}

export function transitionHandoff(
  handoffId: string,
  status: string,
  context: Record<string, string>,
) {
  return callMemory("transitionHandoff", { handoffId, status, context });
}

export function persistReviewContext(review: Record<string, unknown>) {
  return callMemory("persistReviewContext", { review });
}

export function retrieveReviewContext(
  contextId: string,
  context: Record<string, string>,
) {
  return callMemory("retrieveReviewContext", { contextId, context });
}