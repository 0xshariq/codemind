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