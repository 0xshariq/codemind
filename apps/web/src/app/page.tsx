"use client";

import { FormEvent, useState } from "react";

export default function Home() {
  const [result, setResult] = useState<Record<string, unknown> | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    const form = new FormData(event.currentTarget);
    const response = await fetch("/api/relationships", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        symbolId: form.get("symbolId"),
        context: {
          repositoryId: form.get("repositoryId"),
          workspaceId: form.get("workspaceId"),
          sourceSnapshotId: form.get("sourceSnapshotId"),
          graphVersion: form.get("graphVersion"),
        },
      }),
    });
    const payload = (await response.json()) as Record<string, unknown>;
    if (!response.ok) {
      setError(typeof payload.error === "string" ? payload.error : "Query failed");
    }
    else setResult(payload);
  }

  return (
    <main className="mx-auto flex w-full max-w-3xl flex-col gap-8 p-8">
      <header>
        <p className="text-sm font-medium text-zinc-500">CodeMind Day 1</p>
        <h1 className="text-3xl font-semibold">Query persisted code relationships</h1>
      </header>
      <form onSubmit={submit} className="grid gap-3 rounded-xl border p-5">
        {[
          ["symbolId", "Symbol ID"],
          ["repositoryId", "Repository ID"],
          ["workspaceId", "Workspace ID"],
          ["sourceSnapshotId", "Source snapshot ID"],
          ["graphVersion", "Graph version"],
        ].map(([name, label]) => (
          <label key={name} className="grid gap-1 text-sm">
            {label}
            <input name={name} required className="rounded border px-3 py-2" />
          </label>
        ))}
        <button type="submit" className="rounded bg-black px-4 py-2 text-white">
          Query Rust engine
        </button>
      </form>
      {error && <p role="alert" className="rounded border border-red-300 p-4 text-red-700">{error}</p>}
      {result && <pre className="overflow-auto rounded-xl bg-zinc-950 p-5 text-sm text-white">{JSON.stringify(result, null, 2)}</pre>}
    </main>
  );
}
