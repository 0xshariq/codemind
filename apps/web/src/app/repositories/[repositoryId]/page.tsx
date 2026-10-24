"use client";

import { use, useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { ArrowLeft, ExternalLink, FolderGit2, GitBranch, LoaderCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

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
  limitations: string[];
};

type State = { kind: "loading" } | { kind: "ready"; repository: Repository } | { kind: "not-found" } | { kind: "unavailable" };

function optionalString(record: Record<string, unknown>, key: string) {
  return typeof record[key] === "string" ? record[key] as string : undefined;
}

function parseRepository(value: unknown): Repository | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.repositoryId !== "string" || typeof record.name !== "string") return null;
  return {
    repositoryId: record.repositoryId,
    name: record.name,
    readiness: optionalString(record, "readiness") ?? "unknown",
    description: optionalString(record, "description"),
    source: optionalString(record, "source"),
    url: optionalString(record, "url"),
    language: optionalString(record, "language"),
    sourceSnapshotId: optionalString(record, "sourceSnapshotId"),
    graphVersion: optionalString(record, "graphVersion"),
    fileCount: typeof record.fileCount === "number" && Number.isSafeInteger(record.fileCount) && record.fileCount >= 0 ? record.fileCount : undefined,
    moduleCount: typeof record.moduleCount === "number" && Number.isSafeInteger(record.moduleCount) && record.moduleCount >= 0 ? record.moduleCount : undefined,
    limitations: Array.isArray(record.limitations) ? record.limitations.filter((item): item is string => typeof item === "string") : [],
  };
}

export default function RepositoryPage({ params }: PageProps<"/repositories/[repositoryId]">) {
  const { repositoryId } = use(params);
  const router = useRouter();
  const [state, setState] = useState<State>({ kind: "loading" });
  const [starting, setStarting] = useState(false);
  const [startError, setStartError] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    fetch(`/api/repositories/${encodeURIComponent(repositoryId)}`, { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (response.status === 404) {
          setState({ kind: "not-found" });
          return;
        }
        if (!response.ok) throw new Error("repository unavailable");
        const value = await response.json() as { repository?: unknown; outcome?: string; limitations?: string[] };
        if (value.outcome === "unavailable") throw new Error("repository unavailable");
        const repository = parseRepository(value.repository);
        if (!repository) throw new Error("repository metadata invalid");
        repository.limitations = [...repository.limitations, ...(value.limitations ?? [])];
        setState({ kind: "ready", repository });
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState({ kind: "unavailable" });
      });
    return () => controller.abort();
  }, [repositoryId]);

  async function startInvestigation() {
    if (state.kind !== "ready" || !["ready", "indexed", "partial"].includes(state.repository.readiness.toLowerCase())) return;
    setStarting(true);
    setStartError("");
    try {
      const { repository } = state;
      if (!repository.sourceSnapshotId || !repository.graphVersion) {
        throw new Error("This repository does not have a current source snapshot and graph version.");
      }
      const response = await fetch("/api/conversations", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title: `Investigation · ${repository.name}`,
          context: {
            repositoryId: repository.repositoryId,
            sourceSnapshotId: repository.sourceSnapshotId,
            graphVersion: repository.graphVersion,
          },
        }),
      });
      const payload = await response.json().catch(() => null) as { conversation?: { conversationId?: string }; error?: string } | null;
      const conversationId = payload?.conversation?.conversationId;
      if (!response.ok || !conversationId) throw new Error(payload?.error ?? "A repository conversation could not be created.");
      const query = new URLSearchParams({
        repositoryId: repository.repositoryId,
        sourceSnapshotId: repository.sourceSnapshotId,
        graphVersion: repository.graphVersion,
      });
      router.push(`/conversations/${encodeURIComponent(conversationId)}?${query}`);
    } catch (reason) {
      setStartError(reason instanceof Error ? reason.message : "A repository conversation could not be created.");
      setStarting(false);
    }
  }

  if (state.kind === "loading") return <main className="mx-auto w-full max-w-6xl p-4 sm:p-6 lg:p-8"><LoadingState label="Loading repository" detail="Restoring readiness, source snapshot and graph context." /></main>;

  if (state.kind !== "ready") return <main className="mx-auto w-full max-w-4xl p-4 sm:p-6 lg:p-8">{state.kind === "not-found" ? <Card><CardHeader><CardTitle>Repository not found</CardTitle><CardDescription>This repository is not available in the current workspace.</CardDescription></CardHeader><CardContent><Link href="/repositories" className="text-sm text-primary hover:underline">Back to repositories</Link></CardContent></Card> : <ErrorState title="Repository details unavailable" description="The repository service could not return canonical repository state." action={<Link href="/repositories" className="inline-flex min-h-10 items-center rounded-md border border-border px-4 text-sm hover:bg-surface-alt">Back to repositories</Link>} />}</main>;

  const { repository } = state;
    const ready = ["ready", "indexed", "partial"].includes(repository.readiness.toLowerCase());

  return (
    <main className="mx-auto grid w-full max-w-6xl gap-6 p-4 sm:p-6 lg:p-8">
      <Link href="/repositories" className="inline-flex w-fit items-center gap-2 text-sm text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><ArrowLeft aria-hidden="true" className="size-4" />All repositories</Link>
      <header className="flex flex-col justify-between gap-4 border-b border-border pb-6 sm:flex-row sm:items-start">
        <div className="grid min-w-0 gap-3"><div className="flex items-center gap-3"><FolderGit2 aria-hidden="true" className="size-7 shrink-0 text-primary" /><h1 className="break-words text-3xl font-semibold tracking-tight">{repository.name}</h1></div><p className="text-sm text-muted-foreground">{repository.description ?? "Repository details and preparation state."}</p>{repository.url && <a href={repository.url} target="_blank" rel="noreferrer" className="inline-flex w-fit items-center gap-1 break-all font-mono text-xs text-primary hover:underline">{repository.url}<ExternalLink aria-hidden="true" className="size-3 shrink-0" /></a>}</div>
        <div className="flex flex-wrap items-center gap-3"><Badge variant={ready ? "default" : repository.readiness === "failed" ? "destructive" : "secondary"}>{repository.readiness.replaceAll("_", " ")}</Badge><button type="button" onClick={startInvestigation} disabled={!ready || starting} className="inline-flex min-h-10 items-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50">{starting && <LoaderCircle aria-hidden="true" className="size-4 animate-spin" />}Start investigation</button></div>
      </header>

      {startError && <p role="alert" className="rounded-lg border border-danger/50 bg-danger/10 p-3 text-sm text-danger">{startError}</p>}

      <section className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="grid content-start gap-4">
          <Card><CardHeader><CardTitle>Repository readiness</CardTitle><CardDescription>State supplied by the repository service.</CardDescription></CardHeader><CardContent className="grid gap-4 sm:grid-cols-2">
            <div className="flex items-start gap-3"><span className={`mt-1 size-2.5 rounded-full ${ready ? "bg-success" : "bg-warning"}`} /><div><p className="text-sm font-medium">{ready ? "Ready for analysis" : `Readiness: ${repository.readiness.replaceAll("_", " ")}`}</p><p className="mt-1 text-sm text-muted-foreground">{ready ? "Repository preparation is reported complete." : "Analysis actions remain unavailable until the service reports a ready state."}</p></div></div>
            {repository.language && <div><p className="text-xs text-muted-foreground">Language</p><p className="mt-1 text-sm">{repository.language}</p></div>}
            {repository.fileCount !== undefined && <div><p className="text-xs text-muted-foreground">Indexed files</p><p className="mt-1 text-sm">{repository.fileCount}</p></div>}
            {repository.moduleCount !== undefined && <div><p className="text-xs text-muted-foreground">Modules</p><p className="mt-1 text-sm">{repository.moduleCount}</p></div>}
          </CardContent></Card>

          </div>

        <Card><CardHeader><div className="flex items-center gap-2"><GitBranch aria-hidden="true" className="size-4 text-accent" /><CardTitle>Repository context</CardTitle></div></CardHeader><CardContent className="grid gap-4 text-sm">
          {repository.source && <div><p className="text-xs text-muted-foreground">Source type</p><p className="mt-1">{repository.source}</p></div>}
          {repository.sourceSnapshotId ? <div><p className="text-xs text-muted-foreground">Current snapshot</p><code className="mt-1 block break-all font-mono text-xs">{repository.sourceSnapshotId}</code></div> : <p className="text-muted-foreground">Snapshot context is not available.</p>}
          {repository.graphVersion ? <div><p className="text-xs text-muted-foreground">Graph version</p><code className="mt-1 block break-all font-mono text-xs">{repository.graphVersion}</code></div> : <p className="text-muted-foreground">Graph version is not available.</p>}
        </CardContent></Card>
      </section>

      {repository.limitations.length > 0 && <Card><CardHeader><CardTitle>Limitations</CardTitle></CardHeader><CardContent className="grid gap-2">{repository.limitations.map((limitation, index) => <p key={`${index}-${limitation}`} className="text-sm text-muted-foreground">{limitation}</p>)}</CardContent></Card>}
    </main>
  );
}
