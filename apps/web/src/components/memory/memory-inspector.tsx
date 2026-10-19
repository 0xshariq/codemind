"use client";

import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type Memory = {
  memoryId?: string;
  kind?: string;
  scope?: string;
  lifecycle?: string;
  freshness?: string;
  evidenceValidation?: "verified" | "unverified";
  payloadJson?: string;
  context?: { sourceSnapshotId?: string; graphVersion?: string };
  evidence?: Array<{ path?: string; startLine?: number; endLine?: number; snapshotId?: string }>;
};

export function MemoryInspector({
  context,
  scope = "repository",
  kind = "semantic",
  analysisId,
}: {
  context: Record<string, string>;
  scope?: string;
  kind?: string;
  analysisId?: string;
}) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [state, setState] = useState<"loading" | "ready" | "unavailable">("loading");

  useEffect(() => {
    const controller = new AbortController();
    const query = new URLSearchParams({ ...context, scope, kind, ...(analysisId ? { analysisId } : {}) });
    fetch(`/api/memory?${query}`, { signal: controller.signal })
      .then(async (response) => {
        if (!response.ok) throw new Error("unavailable");
        const value = await response.json() as { memories?: Memory[] };
        setMemories(value.memories ?? []);
        setState("ready");
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState("unavailable");
      });
    return () => controller.abort();
  }, [analysisId, context, kind, scope]);

  return (
    <Card aria-label="Memory Inspector">
      <CardHeader><CardTitle>Memory Inspector</CardTitle></CardHeader>
      <CardContent className="grid gap-3">
        {state === "loading" && <p role="status" className="text-sm text-muted-foreground">Loading persisted memory status…</p>}
        {state === "unavailable" && <p role="status" className="text-sm text-warning">Memory is unavailable; current graph evidence remains authoritative.</p>}
        {state === "ready" && memories.length === 0 && <p className="text-sm text-muted-foreground">No authorized memory matches this repository, snapshot, scope and kind.</p>}
        {memories.map((memory, index) => {
          const freshness = memory.freshness ?? "unknown";
          const stale = freshness === "stale";
          let payload = "";
          try { payload = JSON.stringify(JSON.parse(memory.payloadJson ?? ""), null, 2); } catch { payload = "Stored payload is not readable JSON."; }
          return <article key={memory.memoryId ?? `${memory.kind}-${index}`} className={`grid gap-3 rounded-lg border p-3 ${stale ? "border-warning/40 bg-warning/5" : "border-border bg-surface"}`}>
            <div className="flex flex-wrap items-center gap-2"><code className="break-all text-xs">{memory.memoryId ?? "Memory record"}</code><Badge variant="outline">{memory.kind ?? kind}</Badge><Badge variant="outline">{memory.scope ?? scope} scope</Badge></div>
            <div className="flex flex-wrap gap-2"><Badge variant={memory.lifecycle === "validated" ? "default" : "secondary"}>Lifecycle: {memory.lifecycle ?? "unknown"}</Badge><Badge variant={freshness === "fresh" ? "default" : freshness === "stale" ? "destructive" : "outline"}>Freshness: {freshness}</Badge></div>
            <p className="text-xs text-muted-foreground">Lifecycle and snapshot freshness are independent. This panel reports persisted metadata; it does not promote memory into canonical graph evidence.</p>
            <p className="text-xs text-muted-foreground">Current graph evidence match: <span className={memory.evidenceValidation === "verified" ? "text-success" : "text-warning"}>{memory.evidenceValidation === "verified" ? "verified against this analysis" : "unverified; not used as current evidence"}</span></p>
            <dl className="grid gap-1 text-xs">{memory.context?.sourceSnapshotId && <div><dt className="inline text-muted-foreground">Source snapshot: </dt><dd className="inline break-all font-mono">{memory.context.sourceSnapshotId}</dd></div>}{memory.context?.graphVersion && <div><dt className="inline text-muted-foreground">Graph version: </dt><dd className="inline break-all font-mono">{memory.context.graphVersion}</dd></div>}</dl>
            {memory.evidence?.length ? <div className="grid gap-1"><h3 className="text-xs font-medium">Evidence references ({memory.evidence.length})</h3>{memory.evidence.slice(0, 20).map((item, evidenceIndex) => <p key={`${evidenceIndex}-${item.path}`} className="break-all font-mono text-xs text-muted-foreground">{item.path ?? "Unknown path"}{item.startLine ? `:${item.startLine}${item.endLine && item.endLine !== item.startLine ? `-${item.endLine}` : ""}` : ""} · {item.snapshotId ?? "snapshot unknown"}</p>)}</div> : <p className="text-xs text-muted-foreground">No evidence references were returned.</p>}
            <details><summary className="cursor-pointer text-xs text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">View stored payload</summary><pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-md bg-background p-3 font-mono text-xs">{payload}</pre></details>
          </article>;
        })}
      </CardContent>
    </Card>
  );
}
