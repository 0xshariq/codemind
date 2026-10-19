import "server-only";
import path from "node:path";
import { randomUUID } from "node:crypto";
import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import { getServerConfig } from "../config/env";

export type RustClientConfig = { endpoint: string };

export function getRustClientConfig(env: NodeJS.ProcessEnv = process.env): RustClientConfig {
  const config = getServerConfig(env);
  return { endpoint: config.rustEndpoint };
}

type RpcMethod = (...args: unknown[]) => grpc.ClientUnaryCall;
type DynamicClient = grpc.Client & Record<string, RpcMethod>;
type LoadedService = { client: DynamicClient };

const services = new Map<string, Promise<LoadedService>>();

function service(proto: string, name: string): Promise<LoadedService> {
  const config = getRustClientConfig();
  const key = `${config.endpoint}\0${proto}\0${name}`;
  const existing = services.get(key);
  if (existing) return existing;
  const pending = (async () => {
    const packageDefinition = await protoLoader.load(
      path.resolve(process.cwd(), `../../proto/v1/${proto}.proto`),
      { includeDirs: [path.resolve(process.cwd(), "../../proto")] },
    );
    const loaded = grpc.loadPackageDefinition(packageDefinition) as unknown as {
      codemind: { v1: Record<string, new (endpoint: string, credentials: grpc.ChannelCredentials) => DynamicClient> };
    };
    const Constructor = loaded.codemind?.v1?.[name];
    if (!Constructor) throw new Error(`Rust service ${name} is missing from ${proto}.proto`);
    return { client: new Constructor(config.endpoint, grpc.credentials.createInsecure()) };
  })();
  services.set(key, pending);
  pending.catch(() => services.delete(key));
  return pending;
}

function correlation(operationId = randomUUID()) {
  return { requestId: randomUUID(), traceId: randomUUID(), operationId };
}

async function unary(input: {
  proto: string; service: string; method: string; request: Record<string, unknown>;
  principalId?: string;
}) {
  const loaded = await service(input.proto, input.service);
  const metadata = new grpc.Metadata();
  if (input.principalId) {
    if (!input.principalId.trim() || /[\r\n\u0000]/.test(input.principalId)) throw new Error("Rust principal is invalid");
    metadata.set("x-codemind-principal", input.principalId);
  }
  return new Promise<Record<string, unknown>>((resolve, reject) => {
    const method = loaded.client[input.method];
    if (typeof method !== "function") return reject(new Error(`Rust RPC ${input.service}.${input.method} is unavailable`));
    const callback = (error: grpc.ServiceError | null, response: Record<string, unknown>) => {
      if (error) {
        const normalized = new Error(`Rust RPC ${input.service}.${input.method} failed (code ${error.code})`);
        Object.assign(normalized, { code: error.code, retryable: error.code === grpc.status.UNAVAILABLE || error.code === grpc.status.DEADLINE_EXCEEDED });
        reject(normalized);
      } else resolve(response);
    };
    // Do not apply an application deadline: the Rust pipeline may legitimately
    // take longer than an HTTP request's former short default.
    method.call(loaded.client, input.request, metadata, callback);
  });
}

export async function resolveRepository(input: { sourcePath: string; context: { repositoryId: string; workspaceId: string } }) {
  return unary({ proto: "repository", service: "RepositoryService", method: "resolveRepository", request: {
    sourcePath: input.sourcePath,
    context: { ...input.context, sourceSnapshotId: "pending", graphVersion: "pending" },
    correlation: correlation(),
  } });
}

export async function querySymbolRelationships(input: {
  symbolId: string; limit: number;
  context: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
}) {
  return unary({ proto: "intelligence", service: "IntelligenceService", method: "querySymbolRelationships", request: {
    symbolId: input.symbolId, limit: input.limit, context: input.context, correlation: correlation(),
  } });
}

export async function searchCode(input: {
  query: string; limit: number;
  context: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
}) {
  return unary({ proto: "intelligence", service: "IntelligenceService", method: "searchCode", request: {
    query: input.query, limit: input.limit, context: input.context, correlation: correlation(),
  } });
}

export async function analyzeChange(input: {
  symbolId: string; maxDepth: number;
  context: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
}) {
  return unary({ proto: "intelligence", service: "IntelligenceService", method: "analyzeChange", request: {
    symbolId: input.symbolId, maxDepth: input.maxDepth, context: input.context, correlation: correlation(),
  } });
}

export async function startInvestigation(input: {
  sourcePath: string; symbolName: string;
  context: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
}) {
  return unary({ proto: "operation", service: "OperationService", method: "startInvestigation", request: {
    sourcePath: input.sourcePath, symbolName: input.symbolName, context: input.context, correlation: correlation(),
  } });
}

type MemoryInput = { context: Record<string, string>; scope: string; kind: string; ownerId?: string; limit?: number };
function callMemory(method: string, request: Record<string, unknown>, principalId: string) {
  if (!principalId.trim()) return Promise.reject(new Error("memory principal is required"));
  return unary({ proto: "memory", service: "MemoryService", method, request, principalId });
}

export function persistMemory(memory: Record<string, unknown>, principalId: string) {
  return callMemory("persistMemory", { memory }, principalId);
}
export function retrieveMemory(input: MemoryInput, principalId: string) {
  return callMemory("retrieveMemory", { ...input, limit: Math.max(1, Math.min(input.limit ?? 20, 100)) }, principalId);
}
export function persistHandoff(handoff: Record<string, unknown>, idempotencyKey: string, principalId: string) {
  return callMemory("persistHandoff", { handoff, idempotencyKey }, principalId);
}
export function retrieveHandoff(handoffId: string, context: Record<string, string>, principalId: string) {
  return callMemory("retrieveHandoff", { handoffId, context }, principalId);
}
export function transitionHandoff(handoffId: string, status: string, context: Record<string, string>, principalId: string) {
  return callMemory("transitionHandoff", { handoffId, status, context }, principalId);
}
export function persistReviewContext(review: Record<string, unknown>, principalId: string) {
  return callMemory("persistReviewContext", { review }, principalId);
}
export function retrieveReviewContext(contextId: string, context: Record<string, string>, principalId: string) {
  return callMemory("retrieveReviewContext", { contextId, context }, principalId);
}
