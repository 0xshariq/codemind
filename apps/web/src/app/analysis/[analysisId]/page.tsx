"use client";

import { use, useEffect, useState, type ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { MemoryInspector } from "@/components/memory/memory-inspector";

type Context = {
  repository_id: string;
  workspace_id: string;
  source_snapshot_id: string;
  graph_version: string;
};

type SourceEvidence = {
  path: string;
  start_line: number;
  end_line: number;
  snapshot_id: string;
};

type AnalysisData = {
  selected_entity_id: string;
  impact_paths: Array<{
    entity_id: string;
    classification: string;
    depth: number;
    relationships: Array<{ kind: string; from: string; to: string }>;
  }>;
  test_candidates: Array<{
    test_id: string;
    name: string;
    path: string;
    evidence_path: string;
  }>;
  risk: string;
  risk_assessment?: { rationale: string[]; unknown_factors: string[] } | null;
  impact?: { impacted_entity_ids: string[]; availability: string; truncated: boolean } | null;
  evidence_records: Array<{ id: string; derivation: string; source?: SourceEvidence | null }>;
  limitations: string[];
  truncated: boolean;
};

type AnalysisResult = {
  outcome: string;
  completeness: string;
  freshness: string;
  context: Context;
  data: AnalysisData;
  evidence: SourceEvidence[];
  diagnostics: Array<{ code: string; message: string; path?: string; severity?: string }>;
  limitations: string[];
};

type AnalysisResponse = {
  analysisId: string;
  lifecycle: string;
  result?: AnalysisResult | null;
  events?: Array<{ sequence: number; status: string; message?: string; timestamp: string }>;
  error?: { code: string; message: string; retryable?: boolean };
};

type LoadState =
  | { kind: "loading" }
  | { kind: "ready"; value: AnalysisResponse }
  | { kind: "unavailable" };

function label(value: string) {
  return value.toLowerCase().replaceAll("_", " ");
}

function StateBadge({ value }: { value: string }) {
  const variant = ["success", "complete", "current", "completed"].includes(value.toLowerCase())
    ? "default"
    : ["partial", "incomplete", "truncated", "stale", "degraded"].includes(value.toLowerCase())
      ? "secondary"
      : ["failed", "failure", "unavailable"].includes(value.toLowerCase())
        ? "destructive"
        : "outline";
  return <Badge variant={variant}>{label(value)}</Badge>;
}

function SectionCard({ title, description, children }: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <Card className="border-border bg-card">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>
      <CardContent className="grid gap-3">{children}</CardContent>
    </Card>
  );
}

function EvidenceRow({ evidence }: { evidence: SourceEvidence }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-md border border-border/70 px-3 py-2 text-sm">
      <code className="min-w-0 break-all font-mono text-foreground">{evidence.path}</code>
      <span className="text-muted-foreground">lines {evidence.start_line}–{evidence.end_line}</span>
      <span className="text-xs text-muted-foreground">snapshot <code>{evidence.snapshot_id}</code></span>
    </div>
  );
}

export default function AnalysisPage({ params }: PageProps<"/analysis/[analysisId]">) {
  const { analysisId } = use(params);
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setState({ kind: "loading" });
    fetch(`/api/analysis/${encodeURIComponent(analysisId)}`, { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error("analysis unavailable");
        const value = await response.json() as AnalysisResponse;
        setState({ kind: "ready", value });
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState({ kind: "unavailable" });
      });
    return () => controller.abort();
  }, [analysisId, attempt]);

  if (state.kind === "loading") {
    return <main className="mx-auto grid w-full max-w-7xl gap-5 p-4 sm:p-6" aria-busy="true">
      <div className="h-8 w-64 animate-pulse rounded bg-muted" />
      <div className="h-36 animate-pulse rounded-xl bg-muted" />
      <div className="grid gap-4 lg:grid-cols-2"><div className="h-56 animate-pulse rounded-xl bg-muted" /><div className="h-56 animate-pulse rounded-xl bg-muted" /></div>
      <p className="sr-only" role="status">Loading canonical analysis result</p>
    </main>;
  }

  if (state.kind === "unavailable") {
    return <main className="mx-auto w-full max-w-4xl p-4 sm:p-6">
      <Card role="alert" className="border-border bg-card">
        <CardHeader><CardTitle>Analysis is unavailable</CardTitle>
          <CardDescription>The saved result could not be retrieved. No analysis facts are shown until the service returns a canonical result.</CardDescription>
        </CardHeader>
        <CardContent><button className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={() => { setState({ kind: "loading" }); setAttempt((current) => current + 1); }}>
          Retry
        </button></CardContent>
      </Card>
    </main>;
  }

  const { value } = state;
  const result = value.result;
  const data = result?.data;
  const memoryContext: Record<string, string> = result ? {
    repositoryId: result.context.repository_id,
    workspaceId: result.context.workspace_id,
    sourceSnapshotId: result.context.source_snapshot_id,
    graphVersion: result.context.graph_version,
  } : {};

  return (
    <main className="mx-auto grid w-full max-w-7xl gap-5 p-4 sm:p-6">
      <header className="flex flex-col justify-between gap-3 border-b border-border pb-5 sm:flex-row sm:items-start">
        <div className="grid gap-1">
          <p className="text-sm text-muted-foreground">Investigation</p>
          <h1 className="text-2xl font-semibold tracking-tight">Analysis result</h1>
          <code className="break-all font-mono text-xs text-muted-foreground">{value.analysisId}</code>
        </div>
        <div className="flex flex-wrap gap-2" aria-label="Analysis state">
          <span className="text-xs text-muted-foreground">Operation</span><StateBadge value={value.lifecycle} />
          {result && <><span className="ml-2 text-xs text-muted-foreground">Outcome</span><StateBadge value={result.outcome} />
            <span className="ml-2 text-xs text-muted-foreground">Completeness</span><StateBadge value={result.completeness} />
            <span className="ml-2 text-xs text-muted-foreground">Freshness</span><StateBadge value={result.freshness} /></>}
        </div>
      </header>

      {value.error && <Card role="alert" className="border-border bg-card">
        <CardHeader><CardTitle>Investigation did not complete</CardTitle><CardDescription>{value.error.message}</CardDescription></CardHeader>
      </Card>}

      {result && data ? <>
        <SectionCard title="Investigation summary" description="A concise summary of the structured engine result.">
          <p className="text-sm">Analysis of <code className="break-all font-mono text-primary">{data.selected_entity_id}</code> returned {data.impact_paths.length} impact paths and {data.test_candidates.length} test candidates.</p>
          <p className="text-sm text-muted-foreground">Candidates are not evidence that tests were executed. Risk: <span className="font-medium text-foreground">{label(data.risk)}</span>.</p>
        </SectionCard>

        <SectionCard title="Canonical result" description="Structured engine data is authoritative; generated explanation text does not create findings.">
          <div className="grid gap-2 text-sm sm:grid-cols-2">
            <p>Selected entity <code className="ml-2 break-all font-mono text-primary">{data.selected_entity_id}</code></p>
            <p>Repository <code className="ml-2 break-all font-mono">{result.context.repository_id}</code></p>
            <p>Snapshot <code className="ml-2 break-all font-mono">{result.context.source_snapshot_id}</code></p>
            <p>Graph version <code className="ml-2 break-all font-mono">{result.context.graph_version}</code></p>
          </div>
        </SectionCard>

        <div className="grid gap-4 xl:grid-cols-2">
          <SectionCard title="Change impact" description="Direct and indirect impact are shown from canonical paths.">
            <div className="flex flex-wrap gap-2">
              {data.impact && <Badge variant="outline">{data.impact.impacted_entity_ids.length} supported impacted entities</Badge>}
              <Badge variant="outline">{data.impact_paths.length} supported impact paths</Badge>
              <StateBadge value={data.impact?.availability ?? "unknown"} />
              {data.impact?.truncated && <StateBadge value="truncated" />}
            </div>
            {data.impact_paths.length === 0 ? <p className="text-sm text-muted-foreground">No impact paths were returned.</p> : data.impact_paths.map((path, index) => (
              <div key={`${path.entity_id}-${index}`} className="grid gap-1 rounded-md border border-border/70 px-3 py-2">
                <div className="flex flex-wrap items-center gap-2"><Badge variant="secondary">{label(path.classification)}</Badge><span className="text-xs text-muted-foreground">depth {path.depth}</span><code className="break-all font-mono text-sm">{path.entity_id}</code></div>
                <ol className="grid gap-1 pl-5 text-xs text-muted-foreground">
                  {path.relationships.map((relationship, step) => <li key={`${relationship.from}-${relationship.kind}-${relationship.to}-${step}`}>
                    <code className="font-mono">{relationship.from}</code> <span>{label(relationship.kind)} →</span> <code className="font-mono">{relationship.to}</code>
                  </li>)}
                </ol>
              </div>
            ))}
          </SectionCard>

          <SectionCard title="Risk assessment" description="Risk remains separate from impact and evidence completeness.">
            <div className="flex items-center gap-2"><span className="text-sm">Qualitative risk</span><StateBadge value={data.risk} /></div>
            {data.risk_assessment?.rationale.map((reason, index) => <p key={`${index}-${reason}`} className="text-sm">{reason}</p>)}
            {data.risk_assessment?.unknown_factors.map((reason, index) => <p key={`${index}-${reason}`} className="text-sm text-muted-foreground">Unknown factor: {reason}</p>)}
            {!data.risk_assessment && <p className="text-sm text-muted-foreground">No detailed risk rationale was supplied.</p>}
          </SectionCard>

          <SectionCard title="Test impact" description="Candidates are recommendations, not evidence that tests were run.">
            {data.test_candidates.length === 0 ? <p className="text-sm text-muted-foreground">No test candidates were returned.</p> : data.test_candidates.map((test) => (
              <div key={test.test_id} className="grid gap-1 rounded-md border border-border/70 px-3 py-2">
                <span className="text-sm">{test.name}</span><code className="break-all font-mono text-xs text-muted-foreground">{test.path}</code>
                <span className="text-xs text-muted-foreground">Evidence: <code className="font-mono">{test.evidence_path}</code></span>
              </div>
            ))}
          </SectionCard>

          <SectionCard title="Recommendations" description="Recommended tests and their supporting paths are shown as returned in the canonical result.">
            {data.test_candidates.length === 0 ? <p className="text-sm text-muted-foreground">No recommended tests were returned.</p> : data.test_candidates.map((test) => (
              <div key={`recommendation-${test.test_id}`} className="grid gap-1 rounded-md border border-border/70 px-3 py-2">
                <div className="flex flex-wrap items-center gap-2"><Badge variant="secondary">Recommended test</Badge><span className="text-sm">{test.name}</span></div>
                <code className="break-all font-mono text-xs text-muted-foreground">{test.path}</code>
                <span className="text-xs text-muted-foreground">Supporting path: <code className="font-mono">{test.evidence_path}</code></span>
              </div>
            ))}
          </SectionCard>

          <SectionCard title="Evidence" description="Source references retain their snapshot provenance.">
            {result.evidence.length === 0 ? <p className="text-sm text-muted-foreground">No source references were returned.</p> : result.evidence.map((item, index) => <EvidenceRow key={`${item.snapshot_id}-${item.path}-${item.start_line}-${index}`} evidence={item} />)}
            {data.evidence_records.map((item) => <details key={item.id} className="rounded-md border border-border/70 px-3 py-2">
              <summary className="cursor-pointer text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Evidence derivation</summary>
              <p className="mt-2 text-sm text-muted-foreground">{item.derivation}</p>
              {item.source && <div className="mt-2"><EvidenceRow evidence={item.source} /></div>}
            </details>)}
          </SectionCard>
        </div>

        {(result.limitations.length > 0 || data.limitations.length > 0 || result.diagnostics.length > 0 || data.truncated) && <SectionCard title="Limitations and diagnostics">
          {data.truncated && <p className="text-sm text-amber-500">The canonical analysis reports truncated results.</p>}
          {[...new Set([...result.limitations, ...data.limitations])].map((limitation, index) => <p key={`${index}-${limitation}`} className="text-sm text-muted-foreground">{limitation}</p>)}
          {result.diagnostics.map((diagnostic, index) => <div key={`${diagnostic.code}-${index}`} className="text-sm">
            <StateBadge value={diagnostic.severity ?? "diagnostic"} /> <span className="ml-2">{diagnostic.message}</span>{diagnostic.path && <code className="ml-2 font-mono text-xs">{diagnostic.path}</code>}
          </div>)}
        </SectionCard>}
        <MemoryInspector context={memoryContext} scope="repository" kind="semantic" analysisId={value.analysisId} />
      </> : <Card className="border-border bg-card"><CardHeader><CardTitle>No canonical result is available</CardTitle>
        <CardDescription>The operation state is available, but the investigation has not supplied structured result data yet.</CardDescription></CardHeader>
      </Card>}

      <SectionCard title="Pipeline execution" description="Ordered operation events returned by the service.">
        {!value.events?.length ? <p className="text-sm text-muted-foreground">No pipeline events were returned.</p> : value.events.map((event) => <div key={event.sequence} className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-2 last:border-0">
          <div><p className="text-sm">{event.message ?? label(event.status)}</p><time className="font-mono text-xs text-muted-foreground">{event.timestamp}</time></div><StateBadge value={event.status} />
        </div>)}
      </SectionCard>
    </main>
  );
}
