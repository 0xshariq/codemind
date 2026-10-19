"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { ChatComposer } from "./chat-composer";
import { MessageList, type ChatMessage } from "./message-list";
import { ExecutionTimeline, type TimelineEvent } from "../execution/execution-timeline";
import { EngineAccordion } from "../execution/engine-accordion";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type Canonical = { outcome?: string; completeness?: string; freshness?: string; data?: { selected_entity_id?: string; impact_paths?: Array<{ entity_id: string; classification: string; depth: number; relationships?: Array<{ from: string; kind: string; to: string }> }>; risk?: string; test_candidates?: Array<{ test_id: string; name: string; path: string }> }; evidence?: Array<{ path?: string; start_line?: number; end_line?: number; snapshot_id?: string }> };
type OperationView = { events?: TimelineEvent[]; explanation?: string | null; result?: Canonical | null; error?: { message?: string } | null; lifecycle?: string };

export function ChatPanel({
  repositoryId,
  repositoryName,
  conversationId,
  initialMessages = [],
}: {
  repositoryId: string;
  repositoryName?: string;
  conversationId?: string;
  initialMessages?: ChatMessage[];
}) {
  const [messages, setMessages] = useState<ChatMessage[]>(initialMessages);
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [error, setError] = useState("");
  const [running, setRunning] = useState(false);
  const [analysisHref, setAnalysisHref] = useState("");
  const [canonical, setCanonical] = useState<Canonical | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => () => eventSourceRef.current?.close(), []);

  const submit = useCallback(async (question: string) => {
    setError("");
    setEvents([]);
    const userMessage: ChatMessage = { id: crypto.randomUUID(), role: "user", content: question };
    setMessages((current) => [...current, userMessage]);
    window.dispatchEvent(new Event("codemind:conversations-changed"));
    setRunning(true);
    try {
      const response = await fetch("/api/agent", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ question, repositoryId, conversationId }),
      });
      const payload = await response.json() as { operationId?: string; error?: string };
      if (!response.ok || !payload.operationId) throw new Error(payload.error ?? "Investigation could not be started.");
      window.dispatchEvent(new Event("codemind:conversations-changed"));
      const operationId = payload.operationId;
      setAnalysisHref(`/analysis/${encodeURIComponent(operationId)}`);
      eventSourceRef.current?.close();
      const eventSource = new EventSource(`/api/operations/${encodeURIComponent(operationId)}/events`);
      eventSourceRef.current = eventSource;
      const refresh = async () => {
        try {
          const result = await fetch(`/api/operations/${encodeURIComponent(operationId)}`, { cache: "no-store" });
          if (!result.ok) return;
          const history = await result.json() as OperationView;
          window.dispatchEvent(new Event("codemind:conversations-changed"));
          setEvents(history.events ?? []);
          setCanonical(history.result ?? null);
          if (history.explanation) setMessages((current) => current.some((message) => message.id === `${operationId}:answer`)
            ? current
            : [...current, { id: `${operationId}:answer`, role: "assistant", content: history.explanation! }]);
          if (history.error?.message) setError(history.error.message);
          if (history.lifecycle === "terminal" || ["completed", "partial", "failed"].includes(history.events?.at(-1)?.status ?? "")) {
            eventSource.close();
            if (eventSourceRef.current === eventSource) eventSourceRef.current = null;
            setRunning(false);
          }
        } catch { /* EventSource reconnects; the durable operation remains available for the next refresh. */ }
      };
      eventSource.addEventListener("progress", refresh);
      eventSource.addEventListener("end", refresh);
      eventSource.onerror = () => { void refresh(); };
      void refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Investigation could not be started.");
      setRunning(false);
    }
  }, [repositoryId, conversationId]);

  return <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
    <section aria-label="Investigation conversation" className="grid min-h-96 content-start gap-6">
      <MessageList messages={messages} streaming={running} />
      <ChatComposer repositoryName={repositoryName} onSubmit={submit} disabled={running || !repositoryId} error={error} />
    </section>
    <aside className="grid content-start gap-4"><EngineAccordion events={events.filter((event): event is TimelineEvent & { engine: string; stage: string } => Boolean(event.engine && event.stage)).map((event) => ({ ...event, message: event.message ?? "", timestamp: event.timestamp }))} /><ExecutionTimeline events={events} />{analysisHref && <Link href={analysisHref} className="inline-flex min-h-10 items-center justify-center rounded-md border border-border px-3 text-sm hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Open full analysis</Link>}
      {canonical?.data && <div aria-label="Canonical analysis panels" className="grid gap-3">
        <Card><CardHeader><CardTitle>Canonical summary</CardTitle></CardHeader><CardContent className="grid gap-2 text-sm"><div className="flex flex-wrap gap-2"><Badge variant="outline">{canonical.outcome ?? "outcome unknown"}</Badge><Badge variant="secondary">{canonical.completeness ?? "completeness unknown"}</Badge><Badge variant="outline">{canonical.freshness ?? "freshness unknown"}</Badge></div><p>Selected entity: <code className="break-all">{canonical.data.selected_entity_id ?? "Not provided"}</code></p></CardContent></Card>
        <Card><CardHeader><CardTitle>Impact paths ({canonical.data.impact_paths?.length ?? 0})</CardTitle></CardHeader><CardContent className="grid gap-2">{(canonical.data.impact_paths ?? []).slice(0, 30).map((path, index) => <div key={`${path.entity_id}-${index}`} className="grid gap-1 rounded-md border border-border p-2 text-xs"><div><Badge variant="outline">{path.classification}</Badge> <span>depth {path.depth}</span></div><code className="break-all">{path.entity_id}</code>{path.relationships?.map((edge, edgeIndex) => <p key={`${edge.from}-${edge.kind}-${edge.to}-${edgeIndex}`} className="break-all text-muted-foreground"><code>{edge.from}</code> {edge.kind} → <code>{edge.to}</code></p>)}</div>)}</CardContent></Card>
        <Card><CardHeader><CardTitle>Risk and tests</CardTitle></CardHeader><CardContent className="grid gap-2 text-sm"><p>Risk: {canonical.data.risk ?? "unknown"}</p>{(canonical.data.test_candidates ?? []).slice(0, 20).map((test) => <p key={test.test_id} className="break-all">{test.name} · <code>{test.path}</code></p>)}</CardContent></Card>
        <Card><CardHeader><CardTitle>Source evidence ({canonical.evidence?.length ?? 0})</CardTitle></CardHeader><CardContent className="grid gap-1 text-xs">{(canonical.evidence ?? []).slice(0, 30).map((item, index) => <p key={`${item.snapshot_id}-${item.path}-${index}`} className="break-all"><code>{item.path ?? "Path unavailable"}{item.start_line ? `:${item.start_line}${item.end_line && item.end_line !== item.start_line ? `-${item.end_line}` : ""}` : ""}</code> · {item.snapshot_id ?? "snapshot unknown"}</p>)}</CardContent></Card>
      </div>}
    </aside>
  </div>;
}
