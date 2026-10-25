import { getOperation, isWebOperation } from "./service";
import { streamRustProgress } from "@/lib/server/rust-client/progress-stream";

type OperationContext = { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };

/** Shared operation progress framing for internal consumers. Route handlers remain responsible for auth. */
export function operationStream(input: { operationId: string; context: OperationContext; afterSequence?: number; signal?: AbortSignal }): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder(); const abort = new AbortController(); const relayAbort = () => abort.abort(); input.signal?.addEventListener("abort", relayAbort, { once: true });
  return new ReadableStream({
    async start(controller) {
      let last = input.afterSequence ?? 0;
      try {
        if (isWebOperation(input.operationId)) {
          let idleRounds = 0;
          while (!abort.signal.aborted && idleRounds < 1200) {
            const events = getOperation(input.operationId);
            let emitted = false;
            for (const item of events) { if (item.sequence <= last) continue; last = item.sequence; emitted = true; controller.enqueue(encoder.encode(`id: ${last}\nevent: progress\ndata: ${JSON.stringify({ ...item, operationId: input.operationId })}\n\n`)); }
            const terminal = events.at(-1)?.status === "completed" || events.at(-1)?.status === "partial" || events.at(-1)?.status === "failed";
            if (terminal) break;
            idleRounds = emitted ? 0 : idleRounds + 1;
            await new Promise<void>((resolve) => { const timer = setTimeout(resolve, 250); abort.signal.addEventListener("abort", () => { clearTimeout(timer); resolve(); }, { once: true }); });
          }
        } else {
          for await (const progress of streamRustProgress({ ...input, afterSequence: last, signal: abort.signal })) { if (progress.sequence <= last) continue; last = progress.sequence; controller.enqueue(encoder.encode(`id: ${last}\nevent: progress\ndata: ${JSON.stringify(progress)}\n\n`)); }
        }
        if (!abort.signal.aborted) controller.enqueue(encoder.encode(`event: end\ndata: ${JSON.stringify({ operationId: input.operationId, lastSequence: last })}\n\n`));
      } catch {
        if (!abort.signal.aborted) controller.enqueue(encoder.encode(`event: error\ndata: ${JSON.stringify({ outcome: "unavailable", error: "Operation progress is unavailable." })}\n\n`));
      } finally { input.signal?.removeEventListener("abort", relayAbort); if (!abort.signal.aborted) controller.close(); }
    },
    cancel() { abort.abort(); input.signal?.removeEventListener("abort", relayAbort); },
  });
}
