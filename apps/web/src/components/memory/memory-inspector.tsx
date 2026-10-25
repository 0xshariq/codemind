"use client";

import { useEffect, useMemo, useState } from "react";
import { Database, ShieldCheck, ShieldQuestion } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/common/empty-state";

type Memory = { memoryId?: string; kind?: string; scope?: string; lifecycle?: string; freshness?: string; evidenceValidation?: "verified" | "unverified"; payloadJson?: string; context?: { sourceSnapshotId?: string; graphVersion?: string }; evidence?: Array<{ path?: string; startLine?: number; endLine?: number; snapshotId?: string }> };

export function MemoryInspector({ context, scope = "repository", kind = "semantic", analysisId }: { context: Record<string, string>; scope?: string; kind?: string; analysisId?: string }) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [state, setState] = useState<"loading" | "ready" | "unavailable">("loading");
  const contextKey = useMemo(() => JSON.stringify(Object.fromEntries(Object.entries(context).sort(([a], [b]) => a.localeCompare(b)))), [context]);

  useEffect(() => {
    const controller = new AbortController();
    const query = new URLSearchParams({ ...JSON.parse(contextKey) as Record<string, string>, scope, kind, ...(analysisId ? { analysisId } : {}) });
    fetch(`/api/memory?${query}`, { signal: controller.signal, cache: "no-store" })
      .then(async (response) => { if (!response.ok) throw new Error("unavailable"); const value = await response.json() as { memories?: Memory[] }; setMemories(value.memories ?? []); setState("ready"); })
      .catch((error: unknown) => { if (error instanceof DOMException && error.name === "AbortError") return; setState("unavailable"); });
    return () => controller.abort();
  }, [analysisId, contextKey, kind, scope]);

  return <Card aria-label="Memory Inspector"><CardHeader><CardTitle className="flex items-center gap-2"><Database className="size-5 text-accent" />Memory Inspector</CardTitle><CardDescription>Persisted memory is subordinate to current canonical graph evidence.</CardDescription></CardHeader><CardContent className="grid gap-3">
    {state === "loading" && <p role="status" className="text-sm text-muted-foreground">Loading authorized memory…</p>}
    {state === "unavailable" && <p role="status" className="rounded-lg border border-warning/30 bg-warning/5 p-3 text-sm text-warning">Memory is unavailable. Current graph evidence remains authoritative.</p>}
    {state === "ready" && memories.length === 0 && <EmptyState title="No matching memory" description="No authorized memory matches this repository snapshot and scope." icon={<ShieldQuestion className="size-6" />} />}
    {memories.map((memory, index) => { const freshness = memory.freshness ?? "unknown"; let payload = "Stored payload is not readable JSON."; try { payload = JSON.stringify(JSON.parse(memory.payloadJson ?? ""), null, 2); } catch {} const verified = memory.evidenceValidation === "verified"; return <article key={memory.memoryId ?? `${memory.kind}-${index}`} className="grid gap-3 rounded-xl border border-border bg-surface p-4"><div className="flex flex-wrap items-center gap-2"><code className="break-all text-xs">{memory.memoryId ?? "Memory record"}</code><Badge variant="outline">{memory.kind ?? kind}</Badge><Badge variant="outline">{memory.scope ?? scope}</Badge></div><div className="flex flex-wrap gap-2"><Badge variant={memory.lifecycle === "validated" ? "default" : "secondary"}>{memory.lifecycle ?? "unknown"}</Badge><Badge variant={freshness === "fresh" ? "default" : freshness === "stale" ? "destructive" : "outline"}>{freshness}</Badge><span className={`inline-flex items-center gap-1 text-xs ${verified ? "text-success" : "text-warning"}`}>{verified ? <ShieldCheck className="size-3.5" /> : <ShieldQuestion className="size-3.5" />}{verified ? "Evidence verified" : "Evidence unverified"}</span></div><p className="text-xs text-muted-foreground">Memory lifecycle, freshness and evidence validation are separate dimensions.</p>{memory.evidence?.length ? <div><h3 className="text-xs font-medium">Evidence references</h3><ul className="mt-2 grid gap-1">{memory.evidence.slice(0, 20).map((item, itemIndex) => <li key={`${itemIndex}-${item.path}`} className="break-all font-mono text-xs text-muted-foreground">{item.path ?? "Unknown path"}{item.startLine ? `:${item.startLine}${item.endLine && item.endLine !== item.startLine ? `-${item.endLine}` : ""}` : ""} · {item.snapshotId ?? "snapshot unknown"}</li>)}</ul></div> : <p className="text-xs text-muted-foreground">No evidence references were returned.</p>}<details><summary className="cursor-pointer text-xs text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">View stored payload</summary><pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-background p-3 font-mono text-xs">{payload}</pre></details></article>; })}
  </CardContent></Card>;
}
