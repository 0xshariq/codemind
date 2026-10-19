import { NextResponse } from "next/server";
import { loadAuthenticatedSession } from "@/lib/server/auth/context";
import { authorizeMemoryRequest, authorizeRepositoryContext } from "@/lib/server/auth/authorize";
import { canAccessOperation, getOperation, isWebOperation } from "@/lib/server/operations/service";
import { streamRustProgress } from "@/lib/server/rust-client/progress-stream";

const encoder = new TextEncoder();

function frame(event: string, data: unknown, id?: number) {
  return `${id === undefined ? "" : `id: ${id}\n`}event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
}

async function* streamLocalProgress(operationId: string, afterSequence: number, signal: AbortSignal) {
  let last = afterSequence;
  while (!signal.aborted) {
    const events = getOperation(operationId);
    for (const item of events) {
      if (item.sequence <= last) continue;
      last = item.sequence;
      yield {
        eventId: `${operationId}:${item.sequence}`,
        operationId,
        sequence: item.sequence,
        engine: "agent-runtime",
        stage: item.status,
        timestamp: item.timestamp,
        message: item.message ?? "",
        context: null,
        correlation: null,
      };
    }
    const status = events.at(-1)?.status;
    if (status === "completed" || status === "partial" || status === "failed") return;
    await new Promise<void>((resolve) => {
      const onAbort = () => { clearTimeout(timer); resolve(); };
      const timer = setTimeout(() => {
        signal.removeEventListener("abort", onAbort);
        resolve();
      }, 250);
      signal.addEventListener("abort", onAbort, { once: true });
    });
  }
}

export async function GET(request: Request, { params }: { params: Promise<{ operationId: string }> }) {
  let session;
  try {
    session = loadAuthenticatedSession(request);
  } catch {
    return NextResponse.json({ outcome: "unauthorized", error: "Sign in to view operation progress." }, { status: 401 });
  }

  const { operationId } = await params;
  if (!operationId.trim() || operationId.length > 1_024) {
    return NextResponse.json({ outcome: "invalid", error: "A valid operation ID is required." }, { status: 400 });
  }
  const localOperation = isWebOperation(operationId);
  if (localOperation && !canAccessOperation(operationId, session)) {
    return NextResponse.json({ outcome: "not_found", error: "Operation was not found." }, { status: 404 });
  }
  const url = new URL(request.url);
  const repositoryId = url.searchParams.get("repositoryId") ?? "";
  const sourceSnapshotId = url.searchParams.get("sourceSnapshotId") ?? "";
  const graphVersion = url.searchParams.get("graphVersion") ?? "";
  const context = { repositoryId, workspaceId: session.workspaceId, sourceSnapshotId, graphVersion };
  if (!localOperation && [repositoryId, sourceSnapshotId, graphVersion].some((value) => !value.trim() || value.length > 1_024)) {
    return NextResponse.json({ outcome: "invalid", error: "Complete repository context is required." }, { status: 400 });
  }
  if (!authorizeMemoryRequest(session, session.workspaceId, "repository")) {
    return NextResponse.json({ outcome: "forbidden", error: "You do not have access to this repository scope." }, { status: 403 });
  }
  if (!localOperation && !await authorizeRepositoryContext(session, context)) {
    return NextResponse.json({ outcome: "not_found", error: "Repository snapshot was not found." }, { status: 404 });
  }

  const lastEventId = request.headers.get("last-event-id") ?? url.searchParams.get("afterSequence") ?? "0";
  const afterSequence = Number(lastEventId);
  if (!Number.isSafeInteger(afterSequence) || afterSequence < 0) {
    return NextResponse.json({ outcome: "invalid", error: "Last-Event-ID must be a non-negative sequence." }, { status: 400 });
  }

  const abort = new AbortController();
  let heartbeat: ReturnType<typeof setInterval> | undefined;
  let canceled = false;
  const cancel = () => abort.abort();
  request.signal.addEventListener("abort", cancel, { once: true });
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      let closed = false;
      let lastSent = afterSequence;
      heartbeat = setInterval(() => {
        if (!closed && !canceled) controller.enqueue(encoder.encode(": keep-alive\n\n"));
      }, 15_000);

      void (async () => {
        try {
          const progress = localOperation
            ? streamLocalProgress(operationId, afterSequence, abort.signal)
            : streamRustProgress({ operationId, afterSequence, context, signal: abort.signal });
          for await (const event of progress) {
            if (event.sequence <= lastSent) continue;
            lastSent = event.sequence;
            controller.enqueue(encoder.encode(frame("progress", event, event.sequence)));
          }
          if (!closed) controller.enqueue(encoder.encode(frame("end", { operationId, lastSequence: lastSent })));
        } catch {
          if (!closed && !abort.signal.aborted) controller.enqueue(encoder.encode(frame("error", { outcome: "unavailable", error: "Operation progress is unavailable." })));
        } finally {
          closed = true;
          if (heartbeat) clearInterval(heartbeat);
          request.signal.removeEventListener("abort", cancel);
          if (!abort.signal.aborted) controller.close();
        }
      })();
    },
    cancel() {
      canceled = true;
      abort.abort();
      if (heartbeat) clearInterval(heartbeat);
      request.signal.removeEventListener("abort", cancel);
    },
  });

  return new Response(body, {
    headers: {
      "content-type": "text/event-stream; charset=utf-8",
      "cache-control": "no-cache, no-transform",
      connection: "keep-alive",
      "x-accel-buffering": "no",
    },
  });
}
