import Link from "next/link";
import { ArrowUpRight, GitBranch, GitFork, TriangleAlert } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
  limitations?: readonly string[];
};

export function RepositoryCard({ repository }: { repository: RepositoryCardData }) {
  const readiness = repository.readiness.toLowerCase();
  const variant = readiness === "ready" ? "default" : readiness === "partial" ? "secondary" : "destructive";
  return <Card className="min-w-0 border-border bg-surface">
    <CardHeader className="gap-2">
      <div className="flex flex-wrap items-center justify-between gap-2"><Badge variant={variant}>{readiness.replaceAll("_", " ")}</Badge><span className="text-xs capitalize text-muted-foreground">{repository.source}</span></div>
      <CardTitle className="break-words">{repository.name}</CardTitle>
      <CardDescription className="break-words">{repository.description || "Repository source and readiness are provided by the repository service."}</CardDescription>
    </CardHeader>
    <CardContent className="grid gap-3">
      <div className="grid gap-2 text-xs text-muted-foreground">
        {typeof repository.fileCount === "number" && <p className="flex items-center gap-2"><GitFork aria-hidden="true" className="size-3.5" />{repository.fileCount} captured files</p>}
        {repository.sourceSnapshotId && <p className="min-w-0 break-all"><GitBranch aria-hidden="true" className="mr-1 inline size-3.5" />Snapshot <code>{repository.sourceSnapshotId}</code></p>}
        {repository.graphVersion && <p className="min-w-0 break-all">Graph <code>{repository.graphVersion}</code></p>}
      </div>
      {repository.limitations?.length ? <p className="flex gap-2 text-xs text-warning"><TriangleAlert aria-hidden="true" className="size-4 shrink-0" />{repository.limitations[0]}</p> : null}
      <Link href={`/repositories/${encodeURIComponent(repository.repositoryId)}`} className="inline-flex min-h-10 items-center justify-center gap-2 rounded-md border border-border px-3 text-sm font-medium hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Open repository <ArrowUpRight aria-hidden="true" className="size-4" /></Link>
    </CardContent>
  </Card>;
}
