"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { CheckCircle2, CircleDashed, Clock3, Copy, ExternalLink, FolderGit2, GitBranch, GitFork, Github, LoaderCircle, Star, TriangleAlert } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export type RepositoryCardData = {
  repositoryId: string;
  name: string;
  description?: string;
  url?: string;
  source: string;
  readiness: string;
  sourceSnapshotId?: string;
  graphVersion?: string;
  fileCount?: number;
  moduleCount?: number;
  lastAnalyzedAt?: string;
  limitations?: readonly string[];
};

function meta(readiness: string) {
  const value = readiness.toLowerCase();
  if (["ready", "indexed", "snapshot_resolved"].includes(value)) return { label: "Indexed", Icon: CheckCircle2, tone: "text-success", variant: "default" as const };
  if (value.includes("index")) return { label: "Indexing", Icon: LoaderCircle, tone: "text-primary", variant: "outline" as const };
  if (["failed", "unavailable"].includes(value)) return { label: "Failed", Icon: TriangleAlert, tone: "text-danger", variant: "destructive" as const };
  return { label: "Not Indexed", Icon: CircleDashed, tone: "text-muted-foreground", variant: "secondary" as const };
}

function sourceLabel(source: string) {
  return source.replaceAll("_", " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

export function RepositoryCard({ repository }: { repository: RepositoryCardData }) {
  const state = meta(repository.readiness);
  const Icon = state.Icon;
  const [favorite, setFavorite] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    try { setFavorite(localStorage.getItem(`codemind.repository.favorite.${repository.repositoryId}`) === "true"); } catch { /* optional UI preference */ }
  }, [repository.repositoryId]);

  function toggleFavorite() {
    const next = !favorite;
    setFavorite(next);
    try { localStorage.setItem(`codemind.repository.favorite.${repository.repositoryId}`, String(next)); } catch { /* optional UI preference */ }
  }

  async function copyId() {
    try {
      await navigator.clipboard?.writeText(repository.repositoryId);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch { /* clipboard is optional */ }
  }

  const isGithub = repository.url?.includes("github.com") || repository.source.toLowerCase().includes("github");

  return (
    <Card className="min-w-0 border-border bg-surface transition-colors hover:border-primary/40">
      <CardHeader className="gap-3 pb-3">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            <span className="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary">{isGithub ? <Github className="size-5" /> : <FolderGit2 className="size-5" />}</span>
            <div className="min-w-0"><CardTitle className="truncate text-base">{repository.name}</CardTitle><p className="mt-1 truncate text-xs text-muted-foreground">{repository.url ?? sourceLabel(repository.source)}</p></div>
          </div>
          <div className="flex shrink-0 items-center gap-1"><button type="button" onClick={toggleFavorite} aria-pressed={favorite} aria-label={`${favorite ? "Remove" : "Add"} ${repository.name} favorite`} className={`grid size-8 place-items-center rounded-md ${favorite ? "text-warning" : "text-muted-foreground"} hover:bg-surface-alt`}><Star className={`size-4 ${favorite ? "fill-current" : ""}`} /></button><Badge variant={state.variant}><Icon className={`size-3.5 ${repository.readiness.toLowerCase() === "indexing" ? "animate-spin motion-reduce:animate-none" : ""}`} />{state.label}</Badge></div>
        </div>
      </CardHeader>
      <CardContent className="grid gap-4">
        <p className="min-h-10 text-sm leading-5 text-muted-foreground">{repository.description ?? "Repository source registered with CodeMind."}</p>
        <div className="flex flex-wrap gap-2 text-xs"><Badge variant="outline">{sourceLabel(repository.source)}</Badge>{repository.fileCount !== undefined && <Badge variant="outline">{repository.fileCount.toLocaleString()} files</Badge>}{repository.moduleCount !== undefined && <Badge variant="outline">{repository.moduleCount.toLocaleString()} modules</Badge>}</div>
        <div className="grid gap-1 text-xs text-muted-foreground">{repository.lastAnalyzedAt && <span>Last analyzed {new Date(repository.lastAnalyzedAt).toLocaleString()}</span>}{repository.sourceSnapshotId && <span className="flex min-w-0 items-center gap-1"><GitBranch className="size-3.5 shrink-0" />Snapshot <code className="break-all">{repository.sourceSnapshotId}</code></span>}{repository.graphVersion && <span className="flex min-w-0 items-center gap-1"><GitFork className="size-3.5 shrink-0" />Graph <code className="break-all">{repository.graphVersion}</code></span>}</div>
        {repository.limitations?.length ? <p className="flex gap-2 rounded-lg border border-warning/30 bg-warning/5 p-2 text-xs text-warning"><TriangleAlert className="size-4 shrink-0" />{repository.limitations[0]}</p> : null}
        <div className="flex gap-2"><Link href={`/repositories/${encodeURIComponent(repository.repositoryId)}`} className="inline-flex min-h-10 flex-1 items-center justify-center gap-2 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{state.label === "Indexing" ? <LoaderCircle className="size-4 animate-spin motion-reduce:animate-none" /> : <ExternalLink className="size-4" />}{state.label === "Indexing" ? "View Progress" : "Open"}</Link><button type="button" onClick={() => void copyId()} aria-label={`Copy repository ID for ${repository.name}`} className="grid size-10 place-items-center rounded-md border border-border text-muted-foreground hover:bg-surface-alt hover:text-foreground">{copied ? <CheckCircle2 className="size-4 text-success" /> : <Copy className="size-4" />}</button></div>
      </CardContent>
    </Card>
  );
}
