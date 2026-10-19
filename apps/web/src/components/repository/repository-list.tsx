"use client";

import { useMemo, useState } from "react";
import { EmptyState } from "@/components/common/empty-state";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";
import { RepositoryCard, type RepositoryCardData } from "./repository-card";

export function RepositoryList({
  repositories,
  state = "ready",
  onRetry,
}: {
  repositories: readonly RepositoryCardData[];
  state?: "loading" | "ready" | "error" | "unavailable";
  onRetry?: () => void;
}) {
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => repositories.filter((repository) => `${repository.name} ${repository.source} ${repository.readiness}`.toLowerCase().includes(query.trim().toLowerCase())), [repositories, query]);
  if (state === "loading") return <LoadingState label="Loading repositories" />;
  if (state === "error" || state === "unavailable") return <ErrorState title="Repositories are unavailable" description="Repository state could not be loaded from the repository service." onRetry={onRetry} />;
  return <section className="grid gap-4" aria-label="Repositories">
    <label className="grid gap-1 text-xs text-muted-foreground"><span>Filter repositories</span><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} maxLength={200} placeholder="Search by name or source" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring" /></label>
    {filtered.length === 0 ? <EmptyState title={query ? "No matching repositories" : "No repositories yet"} description={query ? "Try a different repository name or source." : "Add a registered local, example, or public GitHub source to get started."} /> : <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">{filtered.map((repository) => <RepositoryCard key={repository.repositoryId} repository={repository} />)}</div>}
  </section>;
}
