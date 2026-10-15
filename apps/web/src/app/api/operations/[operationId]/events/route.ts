import { operationStream } from "@/lib/server/operations/sse";

export async function GET(_request: Request, { params }: { params: Promise<{ operationId: string }> }) {
  const { operationId } = await params;
  return new Response(operationStream(operationId), {
    headers: { "content-type": "text/event-stream", "cache-control": "no-cache" },
  });
}