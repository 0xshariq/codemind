import { NextResponse } from "next/server";
import { z } from "zod";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import type { AuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest } from "@/lib/server/auth/authorize";
import { listRepositories } from "@/lib/server/repositories/service";
import { prepareRepository } from "@/lib/server/workflows/prepare-repository";

const prepareSchema = z.object({
  sourceType: z.enum(["local", "example", "github"]),
  source: z.string().trim().max(1_024).default(""),
  name: z.string().trim().min(1).max(120).optional(),
}).strict().superRefine((value, context) => {
  if (!value.name) context.addIssue({ code: "custom", path: ["name"], message: "Enter a repository name." });
});

function loadSession(request: Request): AuthenticatedSession | NextResponse {
  try { return loadAuthenticatedSession(request); }
  catch { return NextResponse.json({ outcome: "unauthorized", error: "Sign in to access repositories." }, { status: 401 }); }
}

export async function GET(request: Request) {
  const loaded = loadSession(request);
  if (loaded instanceof NextResponse) return loaded;
  const session = loaded;
  if (!authorizeMemoryRequest(session, session.workspaceId, "repository")) {
    return NextResponse.json({ outcome: "forbidden", error: "Repository scope is not authorized." }, { status: 403 });
  }
  try {
    const result = await listRepositories(session.workspaceId);
    const status = result.outcome === "failed" ? 502 : 200;
    return NextResponse.json(result, { status, headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ outcome: "unavailable", repositories: [], error: "Repository service is unavailable." }, { status: 502 });
  }
}

export async function POST(request: Request) {
  const loaded = loadSession(request);
  if (loaded instanceof NextResponse) return loaded;
  const session = loaded;
  if (!authorizeMemoryRequest(session, session.workspaceId, "repository")) {
    return NextResponse.json({ outcome: "forbidden", error: "Repository scope is not authorized." }, { status: 403 });
  }
  const parsed = prepareSchema.safeParse(await request.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ outcome: "invalid", error: parsed.error.issues[0]?.message ?? "Repository source request is invalid." }, { status: 400 });
  const result = await prepareRepository({ ...parsed.data, session });
  const status = result.outcome === "failed" ? 502 : result.outcome === "unsupported" ? 422 : 200;
  return NextResponse.json(result, { status, headers: { "cache-control": "no-store" } });
}
