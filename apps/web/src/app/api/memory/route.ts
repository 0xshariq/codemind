import { NextResponse } from "next/server";
import { z } from "zod";
import { persistMemory } from "@/lib/server/rust-client/client";
import { retrieveAuthorizedMemory } from "@/lib/server/memory/retrieval";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";

const contextSchema = z.object({
  repositoryId: z.string().min(1),
  workspaceId: z.string().min(1),
  sourceSnapshotId: z.string().min(1),
  graphVersion: z.string().min(1),
});

const writeSchema = z.object({
  memory: z.object({
    memoryId: z.string().min(1),
    kind: z.enum(["semantic", "procedure"]),
    scope: z.enum(["user", "repository", "team"]),
    lifecycle: z.enum(["candidate", "validated", "rejected", "superseded"]),
    freshness: z.enum(["fresh", "stale", "unknown"]),
    payloadJson: z.string().max(16_384),
    context: contextSchema,
    evidence: z.array(z.object({
      path: z.string().min(1),
      startLine: z.number().int().nonnegative(),
      endLine: z.number().int().nonnegative(),
      snapshotId: z.string().min(1),
    })),
    idempotencyKey: z.string().min(1),
    version: z.number().int().nonnegative(),
    createdAt: z.string(),
    updatedAt: z.string(),
  }),
});

export async function GET(request: Request) {
  const url = new URL(request.url);
  const context = contextSchema.safeParse(Object.fromEntries(
    ["repositoryId", "workspaceId", "sourceSnapshotId", "graphVersion"]
      .map((key) => [key, url.searchParams.get(key) ?? ""]),
  ));
  const scope = url.searchParams.get("scope") ?? "";
  const kind = url.searchParams.get("kind") ?? "";
  if (!context.success || !["user", "repository", "team"].includes(scope) || !["semantic", "procedure"].includes(kind)) {
    return NextResponse.json({ outcome: "invalid", error: "valid context, scope, and kind are required" }, { status: 400 });
  }
  let session: ReturnType<typeof loadAuthenticatedSession>;
  try {
    session = loadAuthenticatedSession(request);
  } catch (error) {
    return NextResponse.json(
      { outcome: "unauthorized", error: error instanceof Error ? error.message : "authenticated session is required" },
      { status: 401 },
    );
  }
  try {
    return NextResponse.json({ outcome: "success", memories: await retrieveAuthorizedMemory({ context: context.data, scope, kind, session }) });
  } catch (error) {
    return NextResponse.json({ outcome: "unavailable", error: error instanceof Error ? error.message : "memory service unavailable" }, { status: 502 });
  }
}

export async function POST(request: Request) {
  const parsed = writeSchema.safeParse(await request.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: parsed.error.issues[0]?.message ?? "invalid memory" }, { status: 400 });
  let session: ReturnType<typeof loadAuthenticatedSession>;
  try {
    session = loadAuthenticatedSession(request);
  } catch (error) {
    return NextResponse.json(
      { outcome: "unauthorized", error: error instanceof Error ? error.message : "authenticated session is required" },
      { status: 401 },
    );
  }
  const memory = parsed.data.memory;
  if (
    memory.context.workspaceId !== session.workspaceId
    || !session.allowedScopes.includes(memory.scope)
  ) {
    return NextResponse.json({ outcome: "forbidden", error: "memory scope is not authorized" }, { status: 403 });
  }
  try {
    return NextResponse.json({
      outcome: "success",
      memory: await persistMemory({ ...memory, ownerId: memory.scope === "user" ? session.userId : "" }),
    });
  } catch (error) {
    return NextResponse.json({ outcome: "failed", error: error instanceof Error ? error.message : "memory persistence failed" }, { status: 502 });
  }
}