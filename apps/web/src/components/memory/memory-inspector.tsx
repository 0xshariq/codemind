"use client";

import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type Memory = {
  memoryId?: string;
  kind?: string;
  lifecycle?: string;
  freshness?: string;
  payloadJson?: string;
};

export function MemoryInspector({
  context,
  scope = "repository",
  kind = "semantic",
}: {
  context: Record<string, string>;
  scope?: string;
  kind?: string;
}) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [state, setState] = useState<"loading" | "ready" | "unavailable">("loading");

  useEffect(() => {
    const query = new URLSearchParams({ ...context, scope, kind });
    fetch(`/api/memory?${query}`)
      .then(async (response) => {
        if (!response.ok) throw new Error("unavailable");
        const value = await response.json() as { memories?: Memory[] };
        setMemories(value.memories ?? []);
        setState("ready");
      })
      .catch(() => setState("unavailable"));
  }, [context, kind, scope]);

  return (
    <Card aria-label="Memory Inspector">
      <CardHeader><CardTitle>Memory Inspector</CardTitle></CardHeader>
      <CardContent className="grid gap-3">
        {state === "loading" && <p className="text-sm text-muted-foreground">Checking persisted context...</p>}
        {state === "unavailable" && <p role="status" className="text-sm text-warning">Memory is unavailable; current evidence remains authoritative.</p>}
        {state === "ready" && memories.length === 0 && <p className="text-sm text-muted-foreground">No authorized memory matches this context.</p>}
        {memories.map((memory) => (
          <div key={memory.memoryId} className="grid gap-2 rounded-lg border border-border p-3">
            <div className="flex items-center gap-2">
              <code className="text-xs">{memory.memoryId}</code>
              <Badge variant="outline">{memory.kind}</Badge>
              <Badge variant={memory.freshness === "fresh" ? "default" : "destructive"}>{memory.freshness}</Badge>
            </div>
            <p className="text-xs text-muted-foreground">{memory.lifecycle} memory; validated against the current snapshot before use.</p>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}