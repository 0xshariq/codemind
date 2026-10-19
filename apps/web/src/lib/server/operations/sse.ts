import { streamRustProgress } from "@/lib/server/rust-client/progress-stream";

export function operationStream(input: {
  operationId: string;
  context: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
  afterSequence?: number;
  signal?: AbortSignal;
}): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const abort = new AbortController();
  const relayAbort = () => abort.abort();
  input.signal?.addEventListener("abort", relayAbort, { once: true });
  return new ReadableStream({
    async start(controller) {
      let last = input.afterSequence ?? 0;
      try {
        for await (const progress of streamRustProgress({
          ...input,
          afterSequence: last,
          signal: abort.signal,
        })) {
          if (progress.sequence <= last) continue;
          last = progress.sequence;
          controller.enqueue(encoder.encode(`id: ${last}\nevent: progress\ndata: ${JSON.stringify(progress)}\n\n`));
        }
        if (!abort.signal.aborted) controller.enqueue(encoder.encode(`event: end\ndata: ${JSON.stringify({ operationId: input.operationId, lastSequence: last })}\n\n`));
      } catch {
        if (!abort.signal.aborted) controller.enqueue(encoder.encode("event: error\ndata: {\"outcome\":\"unavailable\",\"error\":\"Operation progress is unavailable.\"}\n\n"));
      } finally {
        input.signal?.removeEventListener("abort", relayAbort);
        if (!abort.signal.aborted) controller.close();
      }
    },
    cancel() {
      abort.abort();
      input.signal?.removeEventListener("abort", relayAbort);
    },
  });
}
