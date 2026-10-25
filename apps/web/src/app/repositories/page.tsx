"use client";

import { useEffect, useMemo, useState, type ReactNode } from "react";
import Link from "next/link";
import { CheckCircle2, CircleDashed, Clock3, Filter, Plus, Search, Server, Sparkles } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { RepositoryCard, type RepositoryCardData } from "@/components/repository/repository-card";
import { EmptyState } from "@/components/common/empty-state";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";

type LoadState = { kind: "loading" } | { kind: "ready"; repositories: RepositoryCardData[] } | { kind: "error" };
type FilterState = "all" | "indexed" | "indexing" | "not-indexed";

function parseRepository(value: unknown): RepositoryCardData | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const item = value as Record<string, unknown>;
  if (typeof item.repositoryId !== "string" || typeof item.name !== "string" || typeof item.source !== "string" || typeof item.readiness !== "string") return null;
  return {
    repositoryId: item.repositoryId,
    name: item.name,
    source: item.source,
    readiness: item.readiness,
    ...(typeof item.description === "string" ? { description: item.description } : {}),
    ...(typeof item.url === "string" ? { url: item.url } : {}),
    ...(typeof item.sourceSnapshotId === "string" ? { sourceSnapshotId: item.sourceSnapshotId } : {}),
    ...(typeof item.graphVersion === "string" ? { graphVersion: item.graphVersion } : {}),
    ...(typeof item.fileCount === "number" ? { fileCount: item.fileCount } : {}),
    ...(typeof item.moduleCount === "number" ? { moduleCount: item.moduleCount } : {}),
    ...(typeof item.lastAnalyzedAt === "string" ? { lastAnalyzedAt: item.lastAnalyzedAt } : {}),
    ...(Array.isArray(item.limitations) ? { limitations: item.limitations.filter((entry): entry is string => typeof entry === "string") } : {}),
  };
}

function readinessGroup(readiness: string): Exclude<FilterState, "all"> {
  const value = readiness.toLowerCase();
  if (["ready", "indexed", "snapshot_resolved"].includes(value)) return "indexed";
  if (value.includes("index")) return "indexing";
  return "not-indexed";
}

export default function RepositoriesPage() {
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<FilterState>("all");
  const [showFilters, setShowFilters] = useState(false);

  function load() {
    setState({ kind: "loading" });
    const controller = new AbortController();
    fetch("/api/repositories", { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error();
        const value = await response.json() as { repositories?: unknown[] };
        setState({ kind: "ready", repositories: (value.repositories ?? []).map(parseRepository).filter((item): item is RepositoryCardData => item !== null) });
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState({ kind: "error" });
      });
    return controller;
  }

  useEffect(() => { const controller = load(); return () => controller.abort(); }, []);

  const repositories = state.kind === "ready" ? state.repositories : [];
  const filtered = useMemo(() => repositories.filter((item) => {
    const matchesQuery = `${item.name} ${item.url ?? ""} ${item.source}`.toLowerCase().includes(query.toLowerCase());
    return matchesQuery && (filter === "all" || readinessGroup(item.readiness) === filter);
  }), [repositories, query, filter]);
  const indexed = repositories.filter((item) => readinessGroup(item.readiness) === "indexed").length;
  const indexing = repositories.filter((item) => readinessGroup(item.readiness) === "indexing").length;
  const notIndexed = repositories.filter((item) => readinessGroup(item.readiness) === "not-indexed").length;

  return (
    <main className="mx-auto grid w-full max-w-[1500px] gap-6 p-4 sm:p-6 lg:p-8">
      <header className="flex flex-col justify-between gap-4 lg:flex-row lg:items-end"><div className="flex items-start gap-4"><span className="hidden size-14 shrink-0 place-items-center rounded-2xl bg-primary/10 text-primary sm:grid"><Server className="size-8" /></span><div><p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Workspace</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Repositories</h1><p className="mt-2 text-sm text-muted-foreground">Manage and analyze your code repositories.</p></div></div><div className="flex flex-wrap gap-2"><label className="relative flex min-h-11 min-w-0 flex-1 items-center gap-2 rounded-lg border border-border bg-surface px-3 sm:w-72 sm:flex-none"><Search className="size-4 text-muted-foreground" /><span className="sr-only">Search repositories</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search repositories..." className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground" /></label><button type="button" onClick={() => setShowFilters((value) => !value)} aria-expanded={showFilters} className="inline-flex min-h-11 items-center gap-2 rounded-lg border border-border px-4 text-sm hover:bg-surface-alt"><Filter className="size-4" />Filter</button><Link href="/repositories/new" className="inline-flex min-h-11 items-center gap-2 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary-hover"><Plus className="size-4" />Add Repository</Link></div></header>
      {showFilters && <div className="flex flex-wrap gap-2 rounded-xl border border-border bg-surface p-3"><FilterButton active={filter === "all"} onClick={() => setFilter("all")}>All ({repositories.length})</FilterButton><FilterButton active={filter === "indexed"} onClick={() => setFilter("indexed")}>Indexed ({indexed})</FilterButton><FilterButton active={filter === "indexing"} onClick={() => setFilter("indexing")}>Indexing ({indexing})</FilterButton><FilterButton active={filter === "not-indexed"} onClick={() => setFilter("not-indexed")}>Not Indexed ({notIndexed})</FilterButton></div>}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4"><Stat icon={Server} value={state.kind === "ready" ? repositories.length : "—"} label="Total Repositories" /><Stat icon={CheckCircle2} value={state.kind === "ready" ? indexed : "—"} label="Indexed" tone="success" /><Stat icon={Clock3} value={state.kind === "ready" ? indexing : "—"} label="Indexing" /><Stat icon={CircleDashed} value={state.kind === "ready" ? notIndexed : "—"} label="Not Indexed" tone="accent" /></section>
      {state.kind === "loading" && <LoadingState label="Loading repositories" />}
      {state.kind === "error" && <ErrorState title="Repositories are unavailable" description="The repository service did not return workspace state." onRetry={load} />}
      {state.kind === "ready" && filtered.length === 0 && <EmptyState title={repositories.length ? "No matching repositories" : "No repositories yet"} description={repositories.length ? "Adjust the search or filter to see other repositories." : "Connect a GitHub, local, or example repository to get started."} action={!repositories.length ? <Link href="/repositories/new" className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground">Add Repository</Link> : undefined} />}
      {state.kind === "ready" && filtered.length > 0 && <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">{filtered.map((repository) => <RepositoryCard key={repository.repositoryId} repository={repository} />)}</div>}

      <section className="rounded-xl border border-border bg-surface p-5 sm:p-6"><div className="flex items-center gap-3"><Sparkles className="size-5 text-accent" /><div><h2 className="font-semibold">Add a New Repository</h2><p className="text-sm text-muted-foreground">Connect a repository source to start analyzing its codebase with CodeMind.</p></div></div><div className="mt-4 grid gap-3 md:grid-cols-3">{[["GitHub Repository", "Connect a public or configured GitHub repository."], ["Local Repository", "Analyze a repository from a server-configured local path."], ["Example Repository", "Try a pre-configured example source."]].map(([title, description]) => <Link key={title} href="/repositories/new" className="group rounded-xl border border-border bg-background/40 p-4 hover:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><p className="font-medium">{title}</p><p className="mt-1 text-sm leading-5 text-muted-foreground">{description}</p><span className="mt-4 inline-flex size-8 items-center justify-center rounded-full bg-surface-alt text-primary transition-transform group-hover:translate-x-1">→</span></Link>)}</div></section>
    </main>
  );
}

function FilterButton({ active, onClick, children }: { active: boolean; onClick: () => void; children: ReactNode }) { return <button type="button" onClick={onClick} className={`rounded-md border px-3 py-1.5 text-xs ${active ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-surface-alt"}`}>{children}</button>; }
function Stat({ icon: Icon, value, label, tone = "primary" }: { icon: typeof Server; value: number | string; label: string; tone?: "primary" | "success" | "accent" }) { const toneClass = tone === "success" ? "bg-success/10 text-success" : tone === "accent" ? "bg-accent/10 text-accent" : "bg-primary/10 text-primary"; return <Card><CardContent className="flex items-center gap-4 p-5"><span className={`grid size-10 place-items-center rounded-lg ${toneClass}`}><Icon className="size-5" /></span><div><p className="text-2xl font-semibold">{value}</p><p className="text-sm text-muted-foreground">{label}</p></div></CardContent></Card>; }
