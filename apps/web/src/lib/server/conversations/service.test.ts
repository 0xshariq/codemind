import { beforeEach, describe, expect, it, vi } from "vitest";

const state = vi.hoisted(() => ({ records: new Map<string, Record<string, unknown>>() }));
vi.mock("server-only", () => ({}));
vi.mock("@/lib/server/rust-client/client", () => ({
  persistReviewContext: vi.fn(async (review: Record<string, unknown>) => {
    state.records.set(String(review.contextId), review);
    return {};
  }),
  retrieveReviewContext: vi.fn(async (contextId: string) => {
    const review = state.records.get(contextId);
    if (!review) throw Object.assign(new Error("not found"), { code: 5 });
    return { review };
  }),
}));
vi.mock("@/lib/server/repositories/service", () => ({
  getRepository: vi.fn(async () => ({ sourceSnapshotId: "snapshot", graphVersion: "graph" })),
}));

import { createConversation, deleteConversation, getConversation, listConversations, saveConversation } from "./service";

const session = { userId: "user", workspaceId: "workspace" } as never;
const context = { repositoryId: "repo", workspaceId: "workspace", sourceSnapshotId: "snapshot", graphVersion: "graph" };

describe("conversation lifecycle", () => {
  beforeEach(() => state.records.clear());

  it("logically deletes saved messages and removes the conversation from listings", async () => {
    const conversation = await createConversation({ session, context, title: "Initial analysis" });
    conversation.messages = [{ id: "message", role: "user", content: "private prompt", createdAt: new Date().toISOString() }];
    conversation.graphInitialized = true;
    conversation.initialEvidenceGathered = true;
    conversation.selectedEntityId = "canonical-entity";
    await saveConversation({ session, conversation });
    await expect(getConversation({ session, context, conversationId: conversation.conversationId })).resolves.toMatchObject({
      graphInitialized: true,
      initialEvidenceGathered: true,
      selectedEntityId: "canonical-entity",
    });

    await expect(deleteConversation({ session, context, conversationId: conversation.conversationId })).resolves.toBe(true);
    await expect(getConversation({ session, context, conversationId: conversation.conversationId })).resolves.toBeNull();
    await expect(listConversations({ session, context })).resolves.toEqual([]);

    const serialized = [...state.records.values()].map((record) => String(record.payloadJson)).join("\n");
    expect(serialized).not.toContain("private prompt");
    expect(serialized).toContain('"deleted":true');
  });
});
