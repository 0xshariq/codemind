"use client";

import { useState } from "react";
import { ChatPanel } from "@/components/chat/chat-panel";
import { MemoryInspector } from "@/components/memory/memory-inspector";
import { HandoffStatus } from "@/components/memory/handoff-status";

export default function Home() {
  const [context, setContext] = useState({
    repositoryId: "",
    workspaceId: "",
    sourceSnapshotId: "",
    graphVersion: "",
  });
  const [symbolId, setSymbolId] = useState("");
  const ready = Object.values(context).every(Boolean) && Boolean(symbolId);

  return (
    <main className="mx-auto flex w-full max-w-6xl flex-col gap-8 p-8">
      <header>
        <p className="text-sm font-medium text-muted-foreground">CodeMind investigation workspace</p>
        <h1 className="text-3xl font-semibold">Understand change impact with evidence</h1>
      </header>
      <section className="grid gap-3 rounded-xl border border-border p-5">
        <h2 className="font-medium">Investigation context</h2>
        {Object.entries(context).map(([key, value]) => (
          <label key={key} className="grid gap-1 text-sm">
            {key}
            <input
              value={value}
              required
              className="rounded-lg border border-border bg-background px-3 py-2"
              onChange={(event) => setContext((current) => ({ ...current, [key]: event.target.value }))}
            />
          </label>
        ))}
        <label className="grid gap-1 text-sm">
          symbolId
          <input value={symbolId} required className="rounded-lg border border-border bg-background px-3 py-2" onChange={(event) => setSymbolId(event.target.value)} />
        </label>
      </section>
      {ready ? <ChatPanel symbolId={symbolId} context={context} /> : <p className="text-sm text-muted-foreground">Enter the persisted repository and symbol context to begin.</p>}
      {ready && (
        <section className="grid gap-6 lg:grid-cols-2">
          <MemoryInspector context={context} />
          <HandoffStatus />
        </section>
      )}
    </main>
  );
}
