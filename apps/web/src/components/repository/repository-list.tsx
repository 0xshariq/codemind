"use client";

import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";
import { RepositoryCard, type RepositoryCardData } from "./repository-card";

export function RepositoryList({ repositories, state = "ready", onRetry }: { repositories: readonly RepositoryCardData[]; state?: "loading" | "ready" | "error" | "unavailable"; onRetry?: () => void }) {
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => repositories.filter((repository) => `${repository.name} ${repository.source} ${repository.readiness} ${repository.url ?? ""}`.toLowerCase().includes(query.trim().toLowerCase())), [repositories, query]);
  if (state === "loading") return <LoadingState label="Loading repositories" detail="Reading repository readiness and snapshot metadata." />;
  if (state === "error" || state === "unavailable") return <ErrorState title="Repositories are unavailable" description="Repository state could not be loaded from the repository service." onRetry={onRetry} />;
  return <section className="grid gap-4" aria-label="Repositories"><label className="relative block"><span className="sr-only">Search repositories</span><Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" /><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} maxLength={200} placeholder="Search repositories..." className="min-h-11 w-full rounded-lg border border-border bg-surface pl-9 pr-3 text-sm outline-none placeholder:text-muted-foreground focus:border-primary focus-visible:ring-2 focus-visible:ring-ring" /></label>{filtered.length === 0 ? <EmptyState title={query ? "No matching repositories" : "No repositories yet"} description={query ? "Try a different repository name or source." : "Add a repository to begin analyzing its codebase."} /> : <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">{filtered.map((repository) => <div key={repository.repositoryId}><RepositoryCard repository={repository} /></div>)}</div>}</section>;
}
