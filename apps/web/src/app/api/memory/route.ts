import { NextResponse } from "next/server";
import { z } from "zod";
import { retrieveAuthorizedMemory } from "@/lib/server/memory/retrieval";
import { persistMemoryCandidate } from "@/lib/server/memory/service";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { getRepository } from "@/lib/server/repositories/service";
import { canAccessOperation, getOperationResult } from "@/lib/server/operations/service";
import { validateCanonicalResult } from "@/lib/server/results/validate";
import { authorizeMemoryOwner } from "@/lib/server/auth/authorize";

const contextSchema = z.object({
  repositoryId: z.string().trim().min(1).max(1_024),
  workspaceId: z.string().trim().min(1).max(1_024),
  sourceSnapshotId: z.string().trim().min(1).max(1_024),
  graphVersion: z.string().trim().min(1).max(1_024),
}).strict();

const writeSchema = z.object({
  memory: z.object({
    memoryId: z.string().min(1),
    kind: z.enum(["semantic", "procedure"]),
    scope: z.enum(["user", "repository", "team"]),
    teamId: z.string().trim().min(1).max(1_024).optional(),
    // API callers may submit a candidate. Only the review workflow can promote it.
    lifecycle: z.literal("candidate"),
    freshness: z.enum(["fresh", "stale", "unknown"]).default("unknown"),
    payloadJson: z.string().max(16_384).refine((value) => {
      try {
        const parsed: unknown = JSON.parse(value);
        return !!parsed && typeof parsed === "object" && !Array.isArray(parsed);
      } catch {
        return false;
      }
    }, "payloadJson must contain a JSON object"),
    context: contextSchema,
    evidence: z.array(z.object({
      path: z.string().trim().min(1).max(2_048).refine((value) =>
        !value.startsWith("/") && !value.split(/[\\/]/).includes(".."),
      "evidence path must be repository relative"),
      startLine: z.number().int().positive(),
      endLine: z.number().int().positive(),
      snapshotId: z.string().trim().min(1).max(1_024),
    }).strict()).max(128),
    idempotencyKey: z.string().trim().min(1).max(256),
    version: z.number().int().nonnegative().default(0),
    createdAt: z.string().datetime().optional(),
    updatedAt: z.string().datetime().optional(),
  }).strict(),
}).strict();

const querySchema = contextSchema.extend({
  scope: z.enum(["user", "repository", "team"]),
  kind: z.enum(["semantic", "procedure"]),
  analysisId: z.string().trim().min(1).max(1_024).optional(),
  ownerId: z.string().trim().min(1).max(1_024).optional(),
}).strict();

function evidenceKey(value: unknown): string | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const item = value as Record<string, unknown>;
  const path = item.path;
  const startLine = item.startLine ?? item.start_line;
  const endLine = item.endLine ?? item.end_line;
  const snapshotId = item.snapshotId ?? item.snapshot_id;
  if (typeof path !== "string" || typeof startLine !== "number" || typeof endLine !== "number" || typeof snapshotId !== "string") return null;
  return `${path}\0${startLine}\0${endLine}\0${snapshotId}`;
}

export async function GET(request: Request) {
  const url = new URL(request.url);
  const parsed = querySchema.safeParse(Object.fromEntries(
    ["repositoryId", "workspaceId", "sourceSnapshotId", "graphVersion", "scope", "kind"]
      .map((key) => [key, url.searchParams.get(key) ?? ""])
      .concat(["analysisId", "ownerId"].filter((key) => url.searchParams.has(key))
        .map((key) => [key, url.searchParams.get(key) ?? ""])),
  ));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: "valid context, scope, and kind are required" }, { status: 400 });
  let session: ReturnType<typeof loadAuthenticatedSession>;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to access memory." }, { status: 401 });
  }
  if (parsed.data.workspaceId !== session.workspaceId || !session.allowedScopes.includes(parsed.data.scope)) {
    return NextResponse.json({ outcome: "forbidden", error: "memory scope is not authorized" }, { status: 403 });
  }
  let repository;
  try { repository = await getRepository(session.workspaceId, parsed.data.repositoryId); }
  catch { return NextResponse.json({ outcome: "unavailable", error: "Repository state is unavailable." }, { status: 502 }); }
  if (!repository) return NextResponse.json({ outcome: "not_found", error: "Repository was not found." }, { status: 404 });
  if (repository.sourceSnapshotId !== parsed.data.sourceSnapshotId || repository.graphVersion !== parsed.data.graphVersion) {
    return NextResponse.json({ outcome: "stale", error: "Memory context does not match the current repository snapshot." }, { status: 409 });
  }
  const ownerId = parsed.data.scope === "user" ? session.userId
    : parsed.data.scope === "repository" ? parsed.data.repositoryId
      : parsed.data.ownerId ?? "";
  if (!authorizeMemoryOwner(session, {
    scope: parsed.data.scope,
    ownerId,
    workspaceId: session.workspaceId,
    repositoryId: parsed.data.repositoryId,
  })) return NextResponse.json({ outcome: "forbidden", error: "memory owner is not authorized" }, { status: 403 });
  try {
    const memories = await retrieveAuthorizedMemory({ context: parsed.data, scope: parsed.data.scope, kind: parsed.data.kind, session, ownerId });
    let currentEvidence: Set<string> | null = null;
    if (parsed.data.analysisId) {
      if (!canAccessOperation(parsed.data.analysisId, session)) return NextResponse.json({ outcome: "not_found", error: "Analysis was not found." }, { status: 404 });
      const canonicalValue = getOperationResult(parsed.data.analysisId)?.canonical;
      if (canonicalValue) {
        validateCanonicalResult(canonicalValue);
        const result = canonicalValue as Record<string, unknown>;
        const resultContext = result.context as Record<string, unknown> | undefined;
        if ((resultContext?.workspaceId ?? resultContext?.workspace_id) !== session.workspaceId
          || (resultContext?.repositoryId ?? resultContext?.repository_id) !== parsed.data.repositoryId
          || (resultContext?.sourceSnapshotId ?? resultContext?.source_snapshot_id) !== parsed.data.sourceSnapshotId
          || (resultContext?.graphVersion ?? resultContext?.graph_version) !== parsed.data.graphVersion) {
          return NextResponse.json({ outcome: "stale", error: "Analysis evidence belongs to another repository snapshot." }, { status: 409 });
        }
        const references = Array.isArray(result.evidence) ? [...result.evidence] : [];
        const data = result.data && typeof result.data === "object" ? result.data as Record<string, unknown> : {};
        if (Array.isArray(data.evidence_records)) for (const record of data.evidence_records) {
          if (!record || typeof record !== "object") continue;
          const source = (record as Record<string, unknown>).source;
          if (source) references.push(source);
        }
        currentEvidence = new Set(references.map(evidenceKey).filter((key): key is string => key !== null));
      }
    }
    const checkedMemories = memories.map((memory) => {
      const refs = Array.isArray((memory as unknown as { evidence?: unknown }).evidence) ? (memory as unknown as { evidence: unknown[] }).evidence : [];
      const verified = currentEvidence !== null && refs.length > 0 && refs.every((reference: unknown) => {
        const key = evidenceKey(reference);
        return key !== null && currentEvidence!.has(key);
      });
      return { ...memory, evidenceValidation: verified ? "verified" : "unverified" };
    });
    return NextResponse.json({ outcome: "success", memories: checkedMemories });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "The memory service is unavailable." }, { status: 502 });
  }
}

export async function POST(request: Request) {
  const parsed = writeSchema.safeParse(await request.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: parsed.error.issues[0]?.message ?? "invalid memory" }, { status: 400 });
  let session: ReturnType<typeof loadAuthenticatedSession>;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to write memory." }, { status: 401 });
  }
  const memory = parsed.data.memory;
  if (
    memory.context.workspaceId !== session.workspaceId
    || !session.allowedScopes.includes(memory.scope)
    || memory.evidence.some((item) => item.snapshotId !== memory.context.sourceSnapshotId || item.endLine < item.startLine)
  ) {
    return NextResponse.json({ outcome: "invalid", error: "memory context, scope, or evidence is invalid" }, { status: 403 });
  }
  const ownerId = memory.scope === "user" ? session.userId
    : memory.scope === "repository" ? memory.context.repositoryId
      : memory.teamId ?? "";
  if (!authorizeMemoryOwner(session, {
    scope: memory.scope,
    ownerId,
    workspaceId: session.workspaceId,
    repositoryId: memory.context.repositoryId,
  })) return NextResponse.json({ outcome: "forbidden", error: "memory owner is not authorized" }, { status: 403 });
  let repository;
  try { repository = await getRepository(session.workspaceId, memory.context.repositoryId); }
  catch { return NextResponse.json({ outcome: "unavailable", error: "Repository state is unavailable." }, { status: 502 }); }
  if (!repository) return NextResponse.json({ outcome: "not_found", error: "Repository was not found." }, { status: 404 });
  if (repository.sourceSnapshotId !== memory.context.sourceSnapshotId || repository.graphVersion !== memory.context.graphVersion) {
    return NextResponse.json({ outcome: "stale", error: "Memory evidence does not match the current repository snapshot." }, { status: 409 });
  }
  try {
    return NextResponse.json({
      outcome: "success",
      memory: await persistMemoryCandidate({ session, candidate: memory, teamId: memory.teamId }),
    }, { status: 201 });
  } catch {
    return NextResponse.json({ outcome: "unavailable", error: "Memory could not be persisted." }, { status: 502 });
  }
}
