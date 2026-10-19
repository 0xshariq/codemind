"use client";

import { useState, type FormEvent } from "react";
import { AlertCircle, GitBranch, HardDrive, LoaderCircle } from "lucide-react";

type SourceType = "github" | "local" | "example";
type PrepareResponse = { outcome: string; repositoryId?: string; message?: string; limitations?: string[] };

export function RepositorySetupForm({ onPrepared }: { onPrepared?: (repositoryId: string) => void }) {
  const [sourceType, setSourceType] = useState<SourceType>("github");
  const [source, setSource] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<PrepareResponse | null>(null);
  const [error, setError] = useState("");

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!name.trim()) {
      setError("Enter a repository name.");
      return;
    }
    setBusy(true);
    setError("");
    setMessage(null);
    try {
      const response = await fetch("/api/repositories", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ sourceType, source: source.trim(), name: name.trim() || undefined }),
      });
      const payload: unknown = await response.json().catch(() => null);
      const result: Partial<PrepareResponse> & { error?: unknown } = payload && typeof payload === "object" && !Array.isArray(payload)
        ? payload as Partial<PrepareResponse> & { error?: unknown }
        : {};
      if (!response.ok) {
        const apiMessage = typeof result.error === "string" ? result.error
          : typeof result.message === "string" ? result.message : "Repository preparation failed.";
        throw new Error(response.status === 401
          ? `${apiMessage} Enable the local development session or sign in, then try again.`
          : apiMessage);
      }
      if (typeof result.outcome !== "string") throw new Error("Repository preparation returned an invalid response.");
      const prepared: PrepareResponse = { ...result, outcome: result.outcome };
      setMessage(prepared);
      if (prepared.repositoryId && (prepared.outcome === "completed" || prepared.outcome === "partial")) onPrepared?.(prepared.repositoryId);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Repository service is unavailable.");
    } finally {
      setBusy(false);
    }
  }

  return <form onSubmit={submit} className="grid gap-5 rounded-xl border border-border bg-surface p-5">
    <fieldset className="grid gap-3" disabled={busy}><legend className="text-sm font-medium">Choose a source</legend>
      <div className="grid gap-2 sm:grid-cols-3">
        {(["github", "local", "example"] as const).map((type) => <label key={type} className={`flex min-h-11 cursor-pointer items-center gap-2 rounded-lg border px-3 text-sm capitalize focus-within:ring-2 focus-within:ring-ring ${sourceType === type ? "border-primary bg-primary/5" : "border-border hover:bg-surface-alt"}`}>
          <input type="radio" name="repository-source" value={type} checked={sourceType === type} onChange={() => setSourceType(type)} />
          {type === "github" ? <GitBranch aria-hidden="true" className="size-4" /> : <HardDrive aria-hidden="true" className="size-4" />}{type === "github" ? "GitHub repository" : type === "local" ? "Local path" : "Demo workspace"}
        </label>)}
      </div>
    </fieldset>
    <label className="grid gap-1 text-sm"><span>{sourceType === "github" ? "GitHub repository URL" : sourceType === "local" ? "Server-side local path" : "Configured example name"}</span><input required value={source} onChange={(event) => setSource(event.target.value)} maxLength={1024} placeholder={sourceType === "github" ? "https://github.com/owner/repository" : sourceType === "local" ? "/workspace/repository" : "Demo Workspace"} disabled={busy} className="min-h-10 rounded-md border border-border bg-background px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60" /></label>
    <label className="grid gap-1 text-sm"><span>Repository name</span><input required value={name} onChange={(event) => setName(event.target.value)} maxLength={120} disabled={busy} className="min-h-10 rounded-md border border-border bg-background px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60" /></label>
    <button type="submit" disabled={busy || !source.trim()} className="inline-flex min-h-11 items-center justify-center gap-2 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50">{busy ? <><LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />Preparing source</> : "Prepare repository"}</button>
    {error && <p role="alert" className="flex gap-2 text-sm text-danger"><AlertCircle aria-hidden="true" className="size-4 shrink-0" />{error}</p>}
    {message && <div role="status" className="grid gap-1 rounded-md border border-border p-3 text-sm"><p>{message.message ?? `Repository preparation ${message.outcome}.`}</p>{message.limitations?.map((item) => <p key={item} className="text-xs text-muted-foreground">{item}</p>)}</div>}
  </form>;
}
