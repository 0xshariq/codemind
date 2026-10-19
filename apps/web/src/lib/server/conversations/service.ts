import "server-only";

import { createHash } from "node:crypto";
import { retrieveReviewContext, persistReviewContext } from "@/lib/server/rust-client/client";
import type { AuthenticatedSession } from "@/lib/server/auth/context";
import { getRepository } from "@/lib/server/repositories/service";

export type ConversationContext = {
  repositoryId: string;
  workspaceId: string;
  sourceSnapshotId: string;
  graphVersion: string;
};
export type ConversationMessage = { id: string; role: "user" | "assistant"; content: string; createdAt: string };
export type Conversation = {
  conversationId: string;
  title: string;
  context: ConversationContext;
  messages: ConversationMessage[];
  graphInitialized?: boolean;
  initialEvidenceGathered?: boolean;
  selectedEntityId?: string;
  createdAt: string;
  updatedAt: string;
};
type IndexItem = Pick<Conversation, "conversationId" | "title" | "createdAt" | "updatedAt">;
type ReviewRecord = {
  contextId: string;
  scope: string;
  ownerId: string;
  session: { sessionId: string; scope: string; context: ConversationContext; startedAt: string; endedAt: string | null };
  turns: Array<{ turnId: string; sessionId: string; role: string; summary: string; context: ConversationContext; createdAt: string }>;
  episode: { episodeId: string; sessionId: string; title: string; context: ConversationContext; createdAt: string };
  findings: unknown[];
  context: ConversationContext;
  updatedAt: string;
  graphInitialized?: boolean;
  initialEvidenceGathered?: boolean;
  selectedEntityId?: string;
  deleted?: boolean;
};

const MAX_CONVERSATIONS = 100;
const MAX_MESSAGES = 100;
const MAX_PAYLOAD_BYTES = 512 * 1024;
const CONVERSATION_METADATA_SUFFIX = ":codemind-meta";
const asRecord = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;

function snake(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(snake);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.entries(value).map(([name, child]) => [
    name.replace(/[A-Z]/g, (character) => `_${character.toLowerCase()}`), snake(child),
  ]));
}
function camel(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(camel);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.entries(value).map(([name, child]) => [
    name.replace(/_([a-z])/g, (_, character: string) => character.toUpperCase()), camel(child),
  ]));
}

function key(session: AuthenticatedSession, id: string) {
  const digest = createHash("sha256").update(`${session.userId}\0${id}`).digest("hex");
  return `web-conversation:${digest}`;
}
function indexKey(session: AuthenticatedSession, context: ConversationContext) {
  const digest = createHash("sha256").update([
    session.userId, context.workspaceId, context.repositoryId, context.sourceSnapshotId, context.graphVersion,
  ].join("\0")).digest("hex");
  return `web-conversation-index:${digest}`;
}

function reviewRecord(response: unknown): ReviewRecord | null {
  const review = asRecord(asRecord(response)?.review);
  if (!review || typeof review.payloadJson !== "string") return null;
  try {
    const record = camel(JSON.parse(review.payloadJson)) as ReviewRecord;
    const metadataTurn = record.turns.find((turn) => turn.role === "system" && turn.turnId.endsWith(CONVERSATION_METADATA_SUFFIX));
    if (metadataTurn) {
      try {
        const metadata = JSON.parse(metadataTurn.summary) as Record<string, unknown>;
        record.graphInitialized = metadata.graphInitialized === true;
        record.initialEvidenceGathered = metadata.initialEvidenceGathered === true;
        if (typeof metadata.selectedEntityId === "string") record.selectedEntityId = metadata.selectedEntityId;
        record.deleted = metadata.deleted === true;
      } catch { /* older conversation metadata is optional */ }
    }
    return record;
  } catch { return null; }
}

async function retrieve(session: AuthenticatedSession, context: ConversationContext, id: string) {
  return retrieveReviewContext(id, { ...context, workspaceId: session.workspaceId }, session.userId);
}

async function persist(record: ReviewRecord) {
  const payloadJson = JSON.stringify(snake(record));
  if (Buffer.byteLength(payloadJson, "utf8") > MAX_PAYLOAD_BYTES) throw new Error("conversation exceeds the persisted size limit");
  return persistReviewContext({
    contextId: record.contextId,
    scope: record.scope,
    payloadJson,
    context: record.context,
    updatedAt: record.updatedAt,
  }, record.ownerId);
}

async function validateContext(session: AuthenticatedSession, context: ConversationContext) {
  if (context.workspaceId !== session.workspaceId) throw new Error("conversation workspace is not authorized");
  const repository = await getRepository(session.workspaceId, context.repositoryId);
  if (!repository) throw new Error("conversation repository was not found");
  if (repository.sourceSnapshotId !== context.sourceSnapshotId || repository.graphVersion !== context.graphVersion) {
    throw new Error("conversation context is stale");
  }
}

function toRecord(input: {
  session: AuthenticatedSession;
  context: ConversationContext;
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  turns: ReviewRecord["turns"];
}): ReviewRecord {
  const context = { ...input.context, workspaceId: input.session.workspaceId };
  return {
    contextId: input.id,
    scope: "user",
    ownerId: input.session.userId,
    session: { sessionId: input.id, scope: "user", context, startedAt: input.createdAt, endedAt: null },
    turns: input.turns,
    episode: { episodeId: input.id, sessionId: input.id, title: input.title, context, createdAt: input.createdAt },
    findings: [],
    context,
    updatedAt: input.updatedAt,
  };
}

function fromRecord(record: ReviewRecord): Conversation {
  const contextRecord = record.context as unknown as Record<string, string>;
  return {
    conversationId: record.session.sessionId,
    title: record.episode.title,
    context: {
      repositoryId: contextRecord.repositoryId ?? contextRecord.repository_id,
      workspaceId: contextRecord.workspaceId ?? contextRecord.workspace_id,
      sourceSnapshotId: contextRecord.sourceSnapshotId ?? contextRecord.source_snapshot_id,
      graphVersion: contextRecord.graphVersion ?? contextRecord.graph_version,
    },
    messages: record.turns.filter((turn) => turn.role === "user" || turn.role === "assistant").map((turn) => ({
      id: turn.turnId,
      role: turn.role as "user" | "assistant",
      content: turn.summary,
      createdAt: turn.createdAt,
    })),
    graphInitialized: record.graphInitialized === true,
    initialEvidenceGathered: record.initialEvidenceGathered === true,
    ...(typeof record.selectedEntityId === "string" ? { selectedEntityId: record.selectedEntityId } : {}),
    createdAt: record.session.startedAt,
    updatedAt: record.updatedAt,
  };
}

async function readIndex(session: AuthenticatedSession, context: ConversationContext): Promise<IndexItem[]> {
  let response: unknown;
  try {
    response = await retrieve(session, context, indexKey(session, context));
  } catch (error) {
    if (asRecord(error)?.code === 5) return [];
    throw error;
  }
  const stored = reviewRecord(response);
  if (!stored) return [];
  if (stored.ownerId !== session.userId) throw new Error("conversation index owner mismatch");
  const summary = stored.turns.find((turn) => turn.role === "system")?.summary;
  if (!summary) return [];
  const value = JSON.parse(summary) as unknown;
  return Array.isArray(value) ? value.slice(0, MAX_CONVERSATIONS) as IndexItem[] : [];
}

async function writeIndex(session: AuthenticatedSession, context: ConversationContext, items: IndexItem[]) {
  const now = new Date().toISOString();
  const id = indexKey(session, context);
  const record = toRecord({ session, context, id, title: "Conversation index", createdAt: now, updatedAt: now, turns: [{
    turnId: `${id}:index`, sessionId: id, role: "system", summary: JSON.stringify(items.slice(0, MAX_CONVERSATIONS)),
    context, createdAt: now,
  }] });
  await persist(record);
}

export async function listConversations(input: { session: AuthenticatedSession; context: ConversationContext }) {
  await validateContext(input.session, input.context);
  const items = await readIndex(input.session, input.context);
  const values = await Promise.all(items.map(async (item) => {
    try {
      const record = reviewRecord(await retrieve(input.session, input.context, key(input.session, item.conversationId)));
      return record?.ownerId === input.session.userId && !record.deleted ? fromRecord(record) : null;
    } catch { return null; }
  }));
  return values.filter((item): item is Conversation => item !== null).sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

export async function getConversation(input: { session: AuthenticatedSession; context: ConversationContext; conversationId: string }) {
  await validateContext(input.session, input.context);
  const record = reviewRecord(await retrieve(input.session, input.context, key(input.session, input.conversationId)));
  return record?.ownerId === input.session.userId && !record.deleted ? fromRecord(record) : null;
}

export async function saveConversation(input: { session: AuthenticatedSession; conversation: Conversation }) {
  const { session, conversation } = input;
  if (conversation.context.workspaceId !== session.workspaceId || conversation.messages.length > MAX_MESSAGES) {
    throw new Error("conversation context or message count is invalid");
  }
  await validateContext(session, conversation.context);
  const id = key(session, conversation.conversationId);
  const context = { ...conversation.context, workspaceId: session.workspaceId };
  const turns: ReviewRecord["turns"] = conversation.messages.map((message) => ({
    turnId: message.id,
    sessionId: conversation.conversationId,
    role: message.role,
    summary: message.content.slice(0, 4_000),
    context,
    createdAt: message.createdAt,
  }));
  turns.push({
    turnId: `${conversation.conversationId}${CONVERSATION_METADATA_SUFFIX}`,
    sessionId: conversation.conversationId,
    role: "system",
    summary: JSON.stringify({
      graphInitialized: conversation.graphInitialized === true,
      initialEvidenceGathered: conversation.initialEvidenceGathered === true,
      ...(conversation.selectedEntityId ? { selectedEntityId: conversation.selectedEntityId } : {}),
    }),
    context,
    createdAt: conversation.updatedAt,
  });
  const record = toRecord({ session, context, id, title: conversation.title.slice(0, 160), createdAt: conversation.createdAt, updatedAt: conversation.updatedAt, turns });
  record.graphInitialized = conversation.graphInitialized === true;
  record.initialEvidenceGathered = conversation.initialEvidenceGathered === true;
  if (conversation.selectedEntityId) record.selectedEntityId = conversation.selectedEntityId;
  await persist(record);
  const item: IndexItem = { conversationId: conversation.conversationId, title: conversation.title.slice(0, 160), createdAt: conversation.createdAt, updatedAt: conversation.updatedAt };
  const items = [item, ...(await readIndex(session, context)).filter((entry) => entry.conversationId !== item.conversationId)].slice(0, MAX_CONVERSATIONS);
  await writeIndex(session, context, items);
  return { ...conversation, context, title: item.title };
}

export async function deleteConversation(input: { session: AuthenticatedSession; context: ConversationContext; conversationId: string }) {
  await validateContext(input.session, input.context);
  const storageId = key(input.session, input.conversationId);
  const existing = reviewRecord(await retrieve(input.session, input.context, storageId));
  if (!existing || existing.ownerId !== input.session.userId || existing.deleted) return false;

  // MemoryService has no delete RPC. Persist a scoped tombstone with message
  // content cleared, and remove the conversation from the user-visible index.
  const turns = existing.turns.map((turn) => ({ ...turn, role: "system", summary: "" }));
  const metadataTurn = turns.find((turn) => turn.turnId.endsWith(CONVERSATION_METADATA_SUFFIX));
  if (metadataTurn) metadataTurn.summary = JSON.stringify({ deleted: true });
  const tombstone: ReviewRecord = {
    ...existing,
    deleted: true,
    graphInitialized: false,
    initialEvidenceGathered: false,
    selectedEntityId: undefined,
    // Reuse the existing turn IDs so MemoryService overwrites each persisted
    // graph node, clearing previously stored message text.
    turns,
    episode: { ...existing.episode, title: "Deleted conversation" },
    findings: [],
    updatedAt: new Date().toISOString(),
  };
  await persist(tombstone);
  const items = (await readIndex(input.session, input.context)).filter((item) => item.conversationId !== input.conversationId);
  await writeIndex(input.session, input.context, items);
  return true;
}

export async function createConversation(input: { session: AuthenticatedSession; context: ConversationContext; title?: string }) {
  const now = new Date().toISOString();
  return saveConversation({
    session: input.session,
    conversation: {
      conversationId: crypto.randomUUID(),
      title: (input.title?.trim() || "New conversation").slice(0, 160),
      context: { ...input.context, workspaceId: input.session.workspaceId },
      messages: [], createdAt: now, updatedAt: now,
    },
  });
}

export async function searchConversations(input: { session: AuthenticatedSession; context: ConversationContext; query: string; limit: number }) {
  const query = input.query.trim().toLocaleLowerCase();
  const conversations = await listConversations(input);
  return conversations.filter((conversation) =>
    `${conversation.title} ${conversation.messages.map((message) => message.content).join(" ")}`.toLocaleLowerCase().includes(query),
  ).slice(0, input.limit).map(({ conversationId, title, createdAt, updatedAt }) => ({ conversationId, title, createdAt, updatedAt }));
}
