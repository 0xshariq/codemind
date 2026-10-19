"use client";

import { useEffect, useMemo, useState, type FormEvent } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { AlertCircle, ArrowUp, Code2, GitBranch, LoaderCircle, Network, TriangleAlert } from "lucide-react";

type Repository = {
  repositoryId: string;
  name: string;
  readiness: string;
  sourceSnapshotId?: string;
  graphVersion?: string;
  limitations?: string[];
};

type RepositoryResponse = {
  outcome?: string;
  repositories?: unknown[];
  limitations?: string[];
};

type RepositoryState =
  | { kind: "loading" }
  | { kind: "ready"; repositories: Repository[]; limitations: string[] }
  | { kind: "unavailable" };

const suggestions = [
  { title: "Analyze a file", question: "What could this change affect?", icon: Code2 },
  { title: "Understand a feature", question: "How does this feature work?", icon: Network },
  { title: "Trace an error", question: "What code paths could cause this behavior?", icon: TriangleAlert },
  { title: "Plan a change", question: "Which related code and tests should I review?", icon: GitBranch },
];

function parseRepository(value: unknown): Repository | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  const id = typeof record.repositoryId === "string" ? record.repositoryId : null;
  const name = typeof record.name === "string" ? record.name : null;
  const readiness = typeof record.readiness === "string" ? record.readiness : "unknown";
  if (!id || !name) return null;
  return {
    repositoryId: id,
    name,
    readiness,
    sourceSnapshotId: typeof record.sourceSnapshotId === "string" ? record.sourceSnapshotId : undefined,
    graphVersion: typeof record.graphVersion === "string" ? record.graphVersion : undefined,
    limitations: Array.isArray(record.limitations) ? record.limitations.filter((item): item is string => typeof item === "string") : [],
  };
}

export default function HomePage() {
  const router = useRouter();
  const [repositoryState, setRepositoryState] = useState<RepositoryState>({ kind: "loading" });
  const [repositoryId, setRepositoryId] = useState("");
  const [question, setQuestion] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    fetch("/api/repositories", { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error("repository service unavailable");
        const payload = await response.json() as RepositoryResponse;
        const repositories = (payload.repositories ?? []).map(parseRepository).filter((repository): repository is Repository => repository !== null);
        setRepositoryState({ kind: "ready", repositories, limitations: payload.limitations ?? [] });
        setRepositoryId((selected) => {
          if (selected) return selected;
          const requestedId = new URLSearchParams(window.location.search).get("repositoryId");
          const preferredId = window.localStorage.getItem("codemind.defaultRepositoryId");
          const availableIds = new Set(repositories.map((repository) => repository.repositoryId));
          if (requestedId && availableIds.has(requestedId)) return requestedId;
          if (preferredId && availableIds.has(preferredId)) return preferredId;
          return repositories[0]?.repositoryId ?? "";
        });
      })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError") return;
        setRepositoryState({ kind: "unavailable" });
      });
    return () => controller.abort();
  }, []);

  const repositories = repositoryState.kind === "ready" ? repositoryState.repositories : [];
  const selectedRepository = useMemo(() => repositories.find((repository) => repository.repositoryId === repositoryId), [repositories, repositoryId]);
  const repositoryCanRun = selectedRepository && ["ready", "partial"].includes(selectedRepository.readiness.toLowerCase());
  const canSubmit = Boolean(repositoryCanRun && question.trim() && !submitting);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || !selectedRepository) return;
    setError("");
    setSubmitting(true);
    try {
      const response = await fetch("/api/agent", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ question: question.trim(), repositoryId: selectedRepository.repositoryId }),
      });
      const payload = await response.json() as { operationId?: string; error?: string };
      if (!response.ok || !payload.operationId) throw new Error(payload.error ?? "The investigation could not be started.");
      router.push(`/analysis/${encodeURIComponent(payload.operationId)}`);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "The investigation could not be started.");
      setSubmitting(false);
    }
  }

  return (
    <main className="mx-auto grid w-full max-w-7xl content-center gap-5 px-4 py-5 sm:px-6 lg:min-h-[calc(100svh-3.5rem)] lg:gap-6 lg:px-10">
      <header className="flex justify-end">
        <label className="flex min-h-10 items-center gap-3 rounded-lg border border-border bg-surface px-3 text-sm">
          <span className="text-muted-foreground">Repository</span>
          {repositoryState.kind === "loading" ? <span role="status" className="text-muted-foreground">Loading…</span>
            : repositoryState.kind === "unavailable" ? <span role="status" className="text-warning">Unavailable</span>
              : repositories.length === 0 ? <span className="text-muted-foreground">None available</span>
                : <select aria-label="Select repository" value={repositoryId} onChange={(event) => setRepositoryId(event.target.value)} className="max-w-56 bg-transparent text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring">
                  {repositories.map((repository) => <option key={repository.repositoryId} value={repository.repositoryId} className="bg-surface text-foreground">{repository.name}</option>)}
                </select>}
        </label>
      </header>

      <section className="mx-auto grid w-full max-w-5xl justify-items-center gap-5 text-center">
        <div className="grid justify-items-center gap-2">
          <div aria-hidden="true" className="grid size-14 place-items-center rounded-2xl border border-primary/30 bg-primary/10 text-primary"><Network className="size-8" /></div>
          <h1 className="text-3xl font-semibold tracking-tight sm:text-5xl">Welcome to Code<span className="text-primary">Mind</span></h1>
          <p className="text-lg font-medium">Your agent for understanding and investigating codebases</p>
          <p className="max-w-3xl text-sm leading-6 text-muted-foreground sm:text-base">Ask a question about code relationships, potential impact, test intelligence and the evidence behind a result.</p>
        </div>

        <div className="grid w-full gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {suggestions.map(({ title, question: prompt, icon: Icon }) => <button key={title} type="button" onClick={() => setQuestion(prompt)} className="grid min-h-28 content-start justify-items-start gap-2 rounded-xl border border-border bg-surface p-4 text-left transition-colors hover:border-primary/70 hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
            <Icon aria-hidden="true" className="size-6 text-primary" />
            <span className="font-medium">{title}</span>
            <span className="text-sm text-muted-foreground">“{prompt}”</span>
          </button>)}
        </div>

        <form onSubmit={submit} className="grid w-full gap-3 rounded-xl border border-border bg-surface p-3 text-left focus-within:border-primary sm:p-4">
          <label htmlFor="investigation-question" className="sr-only">Ask CodeMind about the selected repository</label>
          <textarea id="investigation-question" value={question} onChange={(event) => setQuestion(event.target.value)} placeholder="Ask a question about this codebase. Naming a file or module is optional." rows={3} maxLength={4000} disabled={!repositoryCanRun || submitting} className="max-h-36 min-h-24 resize-y bg-transparent px-2 py-1 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60" />
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3">
            <div aria-live="polite" className="text-sm text-muted-foreground">
              {selectedRepository ? <span>Using <span className="text-foreground">{selectedRepository.name}</span>. Ask in plain language; a file or module name is optional.{selectedRepository.readiness.toLowerCase() === "partial" ? " The source snapshot contains omissions." : ""}</span>
                : repositoryState.kind === "ready" ? <span>Add a GitHub repository URL and name to start asking questions.</span>
                  : repositoryState.kind === "unavailable" ? <span>Repository state could not be loaded.</span>
                    : <span>Checking repository readiness…</span>}
            </div>
            <button type="submit" disabled={!canSubmit} className="inline-flex min-h-11 items-center gap-2 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50">
              {submitting ? <><LoaderCircle aria-hidden="true" className="size-4 animate-spin" />Starting</> : <>Investigate <ArrowUp aria-hidden="true" className="size-4" /></>}
            </button>
          </div>
          {error && <p role="alert" className="flex items-center gap-2 text-sm text-danger"><AlertCircle aria-hidden="true" className="size-4" />{error}</p>}
          {selectedRepository && !repositoryCanRun && <p role="status" className="text-sm text-warning">This repository is not marked ready for analysis yet.</p>}
        </form>

        <div className="flex flex-wrap justify-center gap-3">
          <Link href="/repositories/new" className="inline-flex min-h-10 items-center rounded-lg border border-border bg-surface px-4 text-sm hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Add a repository</Link>
          <Link href="/repositories" className="inline-flex min-h-10 items-center rounded-lg border border-border bg-surface px-4 text-sm hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Browse repositories</Link>
        </div>
        {repositoryState.kind === "ready" && repositoryState.limitations.length > 0 && <ul className="max-w-2xl text-left text-xs text-muted-foreground">{repositoryState.limitations.map((limitation) => <li key={limitation}>{limitation}</li>)}</ul>}
      </section>

      <section aria-label="CodeMind capabilities" className="mx-auto grid w-full max-w-5xl gap-4 border-t border-border pt-4 sm:grid-cols-2 xl:grid-cols-4">
        <div className="grid gap-1"><GitBranch aria-hidden="true" className="size-5 text-accent" /><h2 className="text-sm font-medium">Impact analysis</h2><p className="text-xs text-muted-foreground">Trace supported code relationships.</p></div>
        <div className="grid gap-1"><Code2 aria-hidden="true" className="size-5 text-primary" /><h2 className="text-sm font-medium">Test intelligence</h2><p className="text-xs text-muted-foreground">Review relevant test candidates.</p></div>
        <div className="grid gap-1"><Network aria-hidden="true" className="size-5 text-success" /><h2 className="text-sm font-medium">Change intelligence</h2><p className="text-xs text-muted-foreground">See evidence-backed change paths.</p></div>
        <div className="grid gap-1"><AlertCircle aria-hidden="true" className="size-5 text-warning" /><h2 className="text-sm font-medium">Historical insights</h2><p className="text-xs text-muted-foreground">Shown when supported by repository data.</p></div>
      </section>

    </main>
  );
}
