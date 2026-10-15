import { getOperation } from "./service";

export function operationStream(operationId: string): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream({
    async start(controller) {
      let sent = 0;
      const deadline = Date.now() + 30_000;
      while (Date.now() < deadline) {
        const events = getOperation(operationId);
        for (const value of events.slice(sent)) {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify(value)}\n\n`));
          sent += 1;
        }
        const status = events.at(-1)?.status;
        if (status === "completed" || status === "failed") break;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      controller.close();
    },
  });
}