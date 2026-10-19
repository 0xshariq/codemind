"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { CircleDashed, Clock3, FolderGit2, Plus, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type Repository = {
  repositoryId: string;
  name: string;
  readiness: string;
  description?: string;
  source?: string;
  url?: string;
  language?: string;
  sourceSnapshotId?: string;
  graphVersion?: string;
  fileCount?: number;
  moduleCount?: number;
  lastAnalyzedAt?: string;
};

type LoadState = { kind: "loading" } | { kind: "ready"; repositories: Repository[]; limitations: string[] } | { kind: "unavailable" };

function parseRepository(value: unknown): Repository | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const source = value as Record<string, unknown>;
  if (typeof source.repositoryId !== "string" || typeof source.name !== "string") return null;
  const repository: Repository = {
    repositoryId: source.repositoryId,
    name: source.name,
    readiness: typeof source.readiness === "string" ? source.readiness : "unknown",
  };
  for (const key of ["description", "source", "url", "language", "sourceSnapshotId", "graphVersion", "lastAnalyzedAt"] as const) {
    if (typeof source[key] === "string") repository[key] = source[key];
  }
  for (const key of ["fileCount", "moduleCount"] as const) {
    if (typeof source[key] === "number" && Number.isSafeInteger(source[key]) && source[key] >= 0) repository[key] = source[key];
  }
  return repository;
}

function Readiness({ value }: { value: string }) {
  const ready = value.toLowerCase() === "ready" || value.toLowerCase() === "indexed";
  const variant = ready ? "default" : ["failed", "unavailable"].includes(value.toLowerCase()) ? "destructive" : "secondary";
  return <Badge variant={variant}>{value.replaceAll("_", " ")}</Badge>;
}

export default function RepositoriesPage() {
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [query, setQuery] = useState("");
  const [readiness, setReadiness] = useState("all");

  useEffect(() => {
    const controller = new AbortController();
    fetch("/api/repositories", { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error("unavailable");
        const value = await response.json() as { repositories?: unknown[]; limitations?: string[] };
        setState({ kind: "ready", repositories: (value.repositories ?? []).map(parseRepository).filter((item): item is Repository => item !== null), limitations: value.limitations ?? [] });
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState({ kind: "unavailable" });
      });
    return () => controller.abort();
  }, []);

  const repositories = state.kind === "ready" ? state.repositories : [];
  const indexedCount = repositories.filter((repository) => ["ready", "indexed"].includes(repository.readiness.toLowerCase())).length;
  const indexingCount = repositories.filter((repository) => repository.readiness.toLowerCase() === "indexing").length;
  const notIndexedCount = repositories.length - indexedCount - indexingCount;
  const filtered = useMemo(() => repositories.filter((repository) => {
    const matchesQuery = `${repository.name} ${repository.description ?? ""} ${repository.url ?? ""}`.toLowerCase().includes(query.toLowerCase());
    return matchesQuery && (readiness === "all" || repository.readiness.toLowerCase() === readiness);
  }), [repositories, query, readiness]);

  return (
    <main className="mx-auto grid w-full max-w-7xl gap-6 p-4 sm:p-6 lg:p-8">
      <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div><p className="text-sm text-muted-foreground">WORKSPACE</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Repositories</h1><p className="mt-2 text-sm text-muted-foreground">Manage repository sources and review their canonical readiness state.</p></div>
        <Link href="/repositories/new" className="inline-flex min-h-11 items-center justify-center gap-2 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><Plus aria-hidden="true" className="size-4" />Add repository</Link>
      </header>

      <section aria-label="Repository summary" className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <Card><CardContent className="flex items-center gap-4 p-5"><FolderGit2 aria-hidden="true" className="size-7 text-primary" /><div><p className="text-2xl font-semibold">{state.kind === "ready" ? repositories.length : "—"}</p><p className="text-sm text-muted-foreground">{state.kind === "ready" ? "Configured repositories" : "Repository count unavailable"}</p></div></CardContent></Card>
        <Card><CardContent className="flex items-center gap-4 p-5"><span aria-hidden="true" className="size-3 rounded-full bg-success" /><div><p className="text-2xl font-semibold">{state.kind === "ready" ? indexedCount : "—"}</p><p className="text-sm text-muted-foreground">Ready for analysis</p></div></CardContent></Card>
        <Card><CardContent className="flex items-center gap-4 p-5"><Clock3 aria-hidden="true" className="size-7 text-primary" /><div><p className="text-2xl font-semibold">{state.kind === "ready" ? indexingCount : "—"}</p><p className="text-sm text-muted-foreground">Indexing</p></div></CardContent></Card>
        <Card><CardContent className="flex items-center gap-4 p-5"><CircleDashed aria-hidden="true" className="size-7 text-accent" /><div><p className="text-2xl font-semibold">{state.kind === "ready" ? notIndexedCount : "—"}</p><p className="text-sm text-muted-foreground">Not ready</p></div></CardContent></Card>
      </section>

      <section className="grid gap-4">
        <div className="flex flex-col gap-3 sm:flex-row">
          <label className="flex min-h-10 flex-1 items-center gap-2 rounded-lg border border-border bg-surface px-3 focus-within:border-primary">
            <Search aria-hidden="true" className="size-4 text-muted-foreground" /><span className="sr-only">Search repositories</span>
            <input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search repositories" className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground" />
          </label>
          <label className="flex min-h-10 items-center gap-2 rounded-lg border border-border bg-surface px-3 text-sm"><span className="text-muted-foreground">Readiness</span><select value={readiness} onChange={(event) => setReadiness(event.target.value)} className="bg-transparent text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"><option value="all">All states</option><option value="ready">Ready</option><option value="indexed">Indexed</option><option value="indexing">Indexing</option><option value="failed">Failed</option><option value="unknown">Unknown</option></select></label>
        </div>

        {state.kind === "loading" && <div role="status" className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3"><span className="sr-only">Loading repositories</span>{[0, 1, 2].map((item) => <div key={item} className="h-52 animate-pulse rounded-xl bg-surface-alt" />)}</div>}
        {state.kind === "unavailable" && <Card role="alert"><CardHeader><CardTitle>Repositories are unavailable</CardTitle></CardHeader><CardContent><p className="text-sm text-muted-foreground">The repository service could not return workspace state. Try again later or check the service configuration.</p></CardContent></Card>}
        {state.kind === "ready" && filtered.length === 0 && <Card><CardHeader><CardTitle>{repositories.length === 0 ? "No repositories yet" : "No matching repositories"}</CardTitle></CardHeader><CardContent className="grid gap-3"><p className="text-sm text-muted-foreground">{repositories.length === 0 ? "Add a repository to begin exploring its code and test relationships." : "Adjust the search or readiness filter to see other repositories."}</p>{repositories.length === 0 && <Link href="/repositories/new" className="text-sm text-primary hover:underline">Add a repository</Link>}</CardContent></Card>}
        {state.kind === "ready" && filtered.length > 0 && <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">{filtered.map((repository) => <Card key={repository.repositoryId} className="min-w-0">
          <CardHeader className="gap-3"><div className="flex items-start justify-between gap-3"><div className="min-w-0"><CardTitle className="truncate">{repository.name}</CardTitle>{repository.url && <p className="mt-2 break-all font-mono text-xs text-muted-foreground">{repository.url}</p>}</div><Readiness value={repository.readiness} /></div>
            {repository.description && <p className="text-sm text-muted-foreground">{repository.description}</p>}
          </CardHeader>
          <CardContent className="grid gap-4">
            <div className="flex flex-wrap gap-2 text-xs">{repository.source && <Badge variant="outline">{repository.source}</Badge>}{repository.language && <Badge variant="outline">{repository.language}</Badge>}{repository.fileCount !== undefined && <Badge variant="outline">{repository.fileCount} files</Badge>}{repository.moduleCount !== undefined && <Badge variant="outline">{repository.moduleCount} modules</Badge>}</div>
            {(repository.sourceSnapshotId || repository.graphVersion) && <p className="grid gap-1 text-xs text-muted-foreground">{repository.sourceSnapshotId && <span>Snapshot <code className="break-all font-mono">{repository.sourceSnapshotId}</code></span>}{repository.graphVersion && <span>Graph <code className="break-all font-mono">{repository.graphVersion}</code></span>}</p>}
            <div className="grid gap-3 border-t border-border pt-3">
              <span className="text-xs text-muted-foreground">{repository.lastAnalyzedAt ? `Last analyzed ${repository.lastAnalyzedAt}` : "No analysis timestamp supplied"}</span>
              <div className="flex flex-wrap gap-2">
                <Link href={`/repositories/${encodeURIComponent(repository.repositoryId)}`} className="inline-flex min-h-9 items-center justify-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Open</Link>
                {['ready', 'indexed'].includes(repository.readiness.toLowerCase())
                  ? <Link href={`/?repositoryId=${encodeURIComponent(repository.repositoryId)}`} className="inline-flex min-h-9 items-center justify-center rounded-md border border-border bg-surface px-3 text-sm hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Re-analyze</Link>
                  : <span aria-disabled="true" title="Repository must be ready before starting an investigation" className="inline-flex min-h-9 cursor-not-allowed items-center rounded-md border border-border px-3 text-sm text-muted-foreground">Re-analyze unavailable</span>}
              </div>
            </div>
          </CardContent>
        </Card>)}</div>}
        {state.kind === "ready" && state.limitations.length > 0 && <ul className="grid gap-1 text-xs text-muted-foreground">{state.limitations.map((limitation) => <li key={limitation}>{limitation}</li>)}</ul>}
      </section>
    </main>
  );
}
