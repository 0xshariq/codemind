"use client";

import { useEffect, useMemo, useState, type FormEvent } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import {
  AlertCircle,
  ArrowUp,
  Code2,
  FolderOpen,
  GitBranch,
  LoaderCircle,
  Network,
  Plus,
  TriangleAlert,
} from "lucide-react";

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
  { title: "Analyze a file", question: "What could this break?", icon: Code2 },
  { title: "Understand a feature", question: "How does this work?", icon: Network },
  { title: "Trace an error", question: "Find the root cause", icon: TriangleAlert },
  { title: "Plan a change", question: "What needs to be updated?", icon: GitBranch },
];

const capabilities = [
  { title: "Impact Analysis", description: "Find what could break", icon: GitBranch, tone: "text-accent" },
  { title: "Test Intelligence", description: "Discover related tests", icon: Code2, tone: "text-primary" },
  { title: "Change Analysis", description: "Analyze modifications", icon: Network, tone: "text-success" },
  { title: "Historical Insights", description: "Learn from past changes", icon: AlertCircle, tone: "text-warning" },
] as const;

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
    limitations: Array.isArray(record.limitations)
      ? record.limitations.filter((item): item is string => typeof item === "string")
      : [],
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
        const repositories = (payload.repositories ?? [])
          .map(parseRepository)
          .filter((repository): repository is Repository => repository !== null);

        setRepositoryState({
          kind: "ready",
          repositories,
          limitations: payload.limitations ?? [],
        });

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
  const selectedRepository = useMemo(
    () => repositories.find((repository) => repository.repositoryId === repositoryId),
    [repositories, repositoryId],
  );
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
      if (!response.ok || !payload.operationId) {
        throw new Error(payload.error ?? "The investigation could not be started.");
      }
      router.push(`/analysis/${encodeURIComponent(payload.operationId)}`);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "The investigation could not be started.");
      setSubmitting(false);
    }
  }

  return (
    <main className="mx-auto flex min-h-[calc(100svh-1px)] w-full max-w-[1180px] flex-col px-5 py-8 sm:px-8 lg:px-10">
      <header className="flex items-center justify-end">
        <label className="flex h-10 items-center gap-2 rounded-md border border-border bg-surface px-3 text-sm">
          <span className="text-muted-foreground">Select Repository</span>
          {repositoryState.kind === "loading" ? (
            <span role="status" className="text-muted-foreground">Loading…</span>
          ) : repositoryState.kind === "unavailable" ? (
            <span role="status" className="text-warning">Unavailable</span>
          ) : repositories.length === 0 ? (
            <Link href="/repositories/new" className="text-primary hover:underline">Add one</Link>
          ) : (
            <select
              aria-label="Select repository"
              value={repositoryId}
              onChange={(event) => {
                setRepositoryId(event.target.value);
                window.localStorage.setItem("codemind.defaultRepositoryId", event.target.value);
              }}
              className="max-w-56 bg-transparent font-medium text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              {repositories.map((repository) => (
                <option key={repository.repositoryId} value={repository.repositoryId} className="bg-surface text-foreground">
                  {repository.name}
                </option>
              ))}
            </select>
          )}
        </label>
      </header>

      <section className="mx-auto mt-20 grid w-full max-w-[820px] justify-items-center gap-8 text-center lg:mt-24">
        <div className="grid justify-items-center gap-3">
          <div aria-hidden="true" className="grid size-14 place-items-center rounded-xl border border-primary/30 bg-primary/10 text-primary">
            <Network className="size-7" />
          </div>
          <h1 className="text-4xl font-bold tracking-tight sm:text-[48px] sm:leading-[56px]">
            Welcome to Code<span className="text-primary">Mind</span>
          </h1>
          <p className="text-lg font-medium text-foreground">
            Your AI agent for understanding, investigating, and analyzing codebases
          </p>
          <p className="max-w-[720px] text-sm leading-6 text-muted-foreground sm:text-base">
            Ask questions about any public or private repository and get graph-based insights,
            impact analysis, test coverage, and clear explanations.
          </p>
        </div>

        <div className="grid w-full grid-cols-1 gap-3 sm:grid-cols-2">
          {suggestions.map(({ title, question: prompt, icon: Icon }) => (
            <button
              key={title}
              type="button"
              onClick={() => setQuestion(prompt)}
              className="grid min-h-[112px] content-start justify-items-start gap-2 rounded-lg border border-border bg-surface p-4 text-left transition-colors hover:bg-surface-alt hover:border-primary/50 focus-visible:ring-2 focus-visible:ring-ring"
            >
              <Icon aria-hidden="true" className="size-5 text-primary" />
              <span className="font-medium">{title}</span>
              <span className="text-sm text-muted-foreground">“{prompt}”</span>
            </button>
          ))}
        </div>

        <form onSubmit={submit} className="grid w-full gap-0 rounded-xl border border-border bg-surface text-left shadow-sm focus-within:border-primary/70">
          <label htmlFor="investigation-question" className="sr-only">Ask CodeMind anything about your codebase</label>
          <textarea
            id="investigation-question"
            value={question}
            onChange={(event) => setQuestion(event.target.value)}
            placeholder="Ask CodeMind anything about your codebase..."
            rows={4}
            maxLength={4000}
            disabled={!repositoryCanRun || submitting}
            className="min-h-32 resize-none bg-transparent px-4 py-4 text-sm leading-6 outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-60"
          />
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border px-4 py-3">
            <p aria-live="polite" className="text-xs text-muted-foreground">
              {selectedRepository
                ? <>Using <span className="text-foreground">{selectedRepository.name}</span>. Ask in plain language.</>
                : repositoryState.kind === "ready"
                  ? "Select a repository to begin."
                  : repositoryState.kind === "unavailable"
                    ? "Repository state could not be loaded."
                    : "Checking repository readiness…"}
            </p>
            <button
              type="submit"
              disabled={!canSubmit}
              className="inline-flex h-10 items-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary-hover focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
            >
              {submitting ? (
                <>
                  <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />
                  Starting
                </>
              ) : (
                <>
                  Investigate
                  <ArrowUp aria-hidden="true" className="size-4" />
                </>
              )}
            </button>
          </div>
          {error && (
            <p role="alert" className="flex items-center gap-2 border-t border-border px-4 py-3 text-sm text-danger">
              <AlertCircle aria-hidden="true" className="size-4" />
              {error}
            </p>
          )}
        </form>

        <div className="grid w-full grid-cols-1 gap-3 sm:grid-cols-3">
          <Link href="/repositories/new" className="flex min-h-14 items-center justify-center gap-2 rounded-md border border-border bg-surface px-4 text-sm font-medium transition-colors hover:bg-surface-alt focus-visible:ring-2 focus-visible:ring-ring">
            Add GitHub Repository
          </Link>
          <Link href="/repositories/new" className="flex min-h-14 items-center justify-center gap-2 rounded-md border border-border bg-surface px-4 text-sm font-medium transition-colors hover:bg-surface-alt focus-visible:ring-2 focus-visible:ring-ring">
            <FolderOpen aria-hidden="true" className="size-4" />
            Add Local Repository
          </Link>
          <Link href="/repositories/new" className="flex min-h-14 items-center justify-center gap-2 rounded-md border border-border bg-surface px-4 text-sm font-medium transition-colors hover:bg-surface-alt focus-visible:ring-2 focus-visible:ring-ring">
            <Plus aria-hidden="true" className="size-4" />
            Use Example Repository
          </Link>
        </div>
      </section>

      <section aria-labelledby="explore-heading" className="mx-auto mt-auto w-full max-w-[900px] pt-16">
        <div className="mb-5 text-center">
          <h2 id="explore-heading" className="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Explore What You Can Do
          </h2>
        </div>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
          {capabilities.map(({ title, description, icon: Icon, tone }) => (
            <div key={title} className="grid gap-2 border-t border-border pt-4">
              <Icon aria-hidden="true" className={`size-5 ${tone}`} />
              <h3 className="text-sm font-medium">{title}</h3>
              <p className="text-xs leading-5 text-muted-foreground">{description}</p>
            </div>
          ))}
        </div>
      </section>

      {repositoryState.kind === "ready" && repositoryState.limitations.length > 0 && (
        <ul className="mx-auto mt-4 max-w-2xl text-left text-xs text-muted-foreground">
          {repositoryState.limitations.map((limitation) => <li key={limitation}>{limitation}</li>)}
        </ul>
      )}
    </main>
  );
}
