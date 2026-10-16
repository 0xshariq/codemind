import "server-only";
import { retrieveMemory } from "@/lib/server/rust-client/client";
import { canReadMemory } from "./policy";
import { classifyFreshness } from "./freshness";

export async function retrieveAuthorizedMemory(input: {
  context: Record<string, string>;
  scope: string;
  kind: string;
}) {
  if (!canReadMemory({ scope: input.scope, requestedScope: input.scope, workspaceId: input.context.workspaceId })) {
    throw new Error("memory scope is not authorized");
  }
  const response = await retrieveMemory(input);
  const memories = Array.isArray(response.memories) ? response.memories : [];
  return memories.map((memory) => {
    const value = memory as Record<string, unknown>;
    const context = (value.context ?? {}) as Record<string, string>;
    return {
      ...value,
      freshness: classifyFreshness(
        context.sourceSnapshotId ?? "",
        input.context.sourceSnapshotId,
        String(value.lifecycle ?? ""),
      ),
    };
  });
}