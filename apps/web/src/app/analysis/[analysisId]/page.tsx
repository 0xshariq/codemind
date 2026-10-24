"use client";

import { use, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  ArrowLeft,
  BrainCircuit,
  Check,
  CheckCircle2,
  ClipboardCopy,
  FileCode2,
  FlaskConical,
  Network,
  ShieldCheck,
  Sparkles,
  Target,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CompletenessNotice } from "@/components/common/completeness-notice";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";
import { ExecutionTimeline, type TimelineEvent } from "@/components/execution/execution-timeline";
import { EngineAccordion } from "@/components/execution/engine-accordion";
import { OperationStatus } from "@/components/execution/operation-status";
import { SummaryPanel } from "@/components/analysis/summary-panel";
import { ImpactCard } from "@/components/analysis/impact-card";
import { RiskCard } from "@/components/analysis/risk-card";
import { AffectedFilesPanel, type AffectedFile } from "@/components/analysis/affected-files-panel";
import { TestImpactPanel, type TestImpactItem } from "@/components/analysis/test-impact-panel";
import { RecommendationsPanel, type Recommendation } from "@/components/analysis/recommendations-panel";
import { DependencyGraph, type GraphEdge } from "@/components/graph/dependency-graph";
import { GraphPathPanel } from "@/components/graph/graph-path-panel";
import { EvidencePanel, type EvidenceItem } from "@/components/evidence/evidence-panel";
import { HandoffStatus } from "@/components/memory/handoff-status";
import { MemoryInspector } from "@/components/memory/memory-inspector";

type Context = { repository_id: string; workspace_id: string; source_snapshot_id: string; graph_version: string };
type Relationship = { kind: string; from: string; to: string; direction?: string };
type AnalysisData = {
  selected_entity_id?: string;
  impact_paths?: Array<{ entity_id: string; classification: string; depth: number; relationships?: Relationship[] }>;
  test_candidates?: Array<{ test_id: string; name: string; path: string; evidence_path?: string; impact?: string }>;
  risk?: string;
  risk_assessment?: { rationale?: string[]; unknown_factors?: string[]; confidence?: number | string } | null;
  impact?: { impacted_entity_ids?: string[]; availability?: string; truncated?: boolean } | null;
  evidence_records?: Array<{ id: string; derivation: string; source?: { path: string; start_line: number; end_line: number; snapshot_id: string } | null }>;
  recommendations?: Array<{ id: string; title: string; detail?: string; priority?: string; evidence?: string[] }>;
  handoff?: { handoff_id?: string; status?: "pending" | "claimed" | "consumed" | "expired"; source_stage?: string; target_stage?: string } | null;
};
type AnalysisResult = { outcome: string; completeness: string; freshness: string; context: Context; data: AnalysisData; evidence: Array<{ path: string; start_line?: number; end_line?: number; snapshot_id: string }>; diagnostics: Array<{ code?: string; message: string; path?: string; severity?: string } | string>; limitations: string[] };
type AnalysisResponse = { analysisId: string; lifecycle: string; answer?: string; result?: AnalysisResult | null; events?: Array<TimelineEvent>; error?: { code: string; message: string; retryable?: boolean } | null; limitations?: string[] };
type LoadState = { kind: "loading" } | { kind: "ready"; value: AnalysisResponse } | { kind: "error"; message: string };

function resultEdges(data: AnalysisData): GraphEdge[] {
  const edges: GraphEdge[] = [];
  for (const path of data.impact_paths ?? []) for (const [index, relation] of (path.relationships ?? []).entries()) {
    edges.push({ id: `${path.entity_id}-${relation.from}-${relation.kind}-${relation.to}-${index}`, from: relation.from, to: relation.to, relationship: relation.kind, depth: path.depth });
  }
  return edges;
}

function pathRows(data: AnalysisData) {
  return (data.impact_paths ?? []).map((path, index) => ({
    id: `${path.entity_id}-${index}`,
    classification: path.classification,
    entities: [...new Set([path.entity_id, ...(path.relationships ?? []).flatMap((item) => [item.from, item.to])])].map((id) => ({ id, label: id })),
    relationships: path.relationships ?? [],
  }));
}

function normalizedRisk(risk?: string) {
  return (risk ?? "unknown").trim().toLowerCase();
}

function isSafeRisk(risk?: string) {
  const value = normalizedRisk(risk);
  return value.includes("none") || value.includes("zero") || value === "low" || value.includes("low risk");
}

function isHighRisk(risk?: string) {
  const value = normalizedRisk(risk);
  return value.includes("high") || value.includes("critical");
}

function riskLabel(risk?: string) {
  return (risk ?? "Unknown").replaceAll("_", " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

function MiniDependencyGraph({ edges }: { edges: readonly GraphEdge[] }) {
  if (!edges.length) {
    return <div className="grid min-h-48 place-items-center rounded-lg border border-dashed border-border bg-background/40 p-6 text-center"><Network className="size-8 text-muted-foreground" /><p className="mt-2 text-sm text-muted-foreground">No supported dependency edges were returned.</p></div>;
  }
  const visible = edges.slice(0, 7);
  const nodes = [...new Set(visible.flatMap((edge) => [edge.from, edge.to]))].slice(0, 8);
  const positions = new Map(nodes.map((id, index) => [id, { x: 150 + (index % 2) * 220, y: 45 + Math.floor(index / 2) * 65 }]));
  return <div className="overflow-hidden rounded-lg border border-border bg-background/40 p-2"><svg role="img" aria-label={`Dependency graph overview with ${visible.length} relationships`} viewBox="0 0 520 300" className="h-64 w-full"><defs><marker id="analysis-mini-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z" className="fill-primary" /></marker></defs>{visible.map((edge) => { const from = positions.get(edge.from); const to = positions.get(edge.to); if (!from || !to) return null; return <line key={edge.id} x1={from.x} y1={from.y} x2={to.x} y2={to.y} className="stroke-primary/60" strokeWidth="1.5" markerEnd="url(#analysis-mini-arrow)" />; })}{nodes.map((id) => { const point = positions.get(id)!; return <g key={id}><rect x={point.x - 68} y={point.y - 18} width="136" height="36" rx="8" className="fill-surface stroke-border" /><text x={point.x} y={point.y + 4} textAnchor="middle" className="fill-foreground" fontSize="10">{id.length > 20 ? `${id.slice(0, 17)}…` : id}</text></g>; })}</svg></div>;
}

export default function AnalysisPage({ params }: PageProps<"/analysis/[analysisId]">) {
  const { analysisId } = use(params);
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [activeTab, setActiveTab] = useState("summary");

  useEffect(() => {
    const controller = new AbortController();
    setState({ kind: "loading" });
    fetch(`/api/analysis/${encodeURIComponent(analysisId)}`, { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        const value = await response.json().catch(() => null) as AnalysisResponse | null;
        if (!response.ok || !value) throw new Error(value?.error?.message ?? "Analysis is unavailable.");
        setState({ kind: "ready", value });
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState({ kind: "error", message: error instanceof Error ? error.message : "Analysis is unavailable." });
      });
    return () => controller.abort();
  }, [analysisId, attempt]);

  if (state.kind === "loading") return <main className="mx-auto w-full max-w-7xl p-4 sm:p-6 lg:p-8"><LoadingState label="Loading analysis workspace" detail="Restoring canonical result, operation progress and evidence context." /></main>;
  if (state.kind === "error") return <main className="mx-auto w-full max-w-3xl p-4 sm:p-6 lg:p-8"><ErrorState title="Analysis is unavailable" description={state.message} onRetry={() => setAttempt((value) => value + 1)} /></main>;

  const { value } = state;
  const result = value.result;
  const data = result?.data ?? {};
  const events = value.events ?? [];
  const context = result?.context;
  const edges = resultEdges(data);
  const paths = pathRows(data);
  const evidence: EvidenceItem[] = (result?.evidence ?? []).map((item, index) => ({ id: `${item.path}-${item.start_line}-${index}`, path: item.path, startLine: item.start_line, endLine: item.end_line, snapshotId: item.snapshot_id }));
  const affectedFiles: AffectedFile[] = [...new Map(evidence.flatMap((item) => item.path ? [[item.path, { id: item.path, path: item.path, relation: "evidence-supported", completeness: result?.completeness, evidence: item.startLine ? `lines ${item.startLine}${item.endLine && item.endLine !== item.startLine ? `-${item.endLine}` : ""}` : undefined }] as const] : [])).values()];
  const tests: TestImpactItem[] = (data.test_candidates ?? []).map((item) => ({ id: item.test_id, name: item.name, path: item.path, evidence: item.evidence_path, snapshotId: context?.source_snapshot_id }));
  const recommendations: Recommendation[] = (data.recommendations ?? []).map((item) => ({ id: item.id, title: item.title, detail: item.detail, priority: item.priority, evidence: item.evidence }));
  const directCount = (data.impact_paths ?? []).filter((item) => item.classification.toLowerCase().includes("direct")).length;
  const indirectCount = (data.impact_paths ?? []).filter((item) => item.classification.toLowerCase().includes("indirect")).length;
  const memoryContext: Record<string, string> = context ? { repositoryId: context.repository_id, workspaceId: context.workspace_id, sourceSnapshotId: context.source_snapshot_id, graphVersion: context.graph_version } : {};
  const summary = value.answer?.trim() || (result ? `The canonical analysis selected ${data.selected_entity_id ?? "an entity"} and returned ${(data.impact_paths ?? []).length} impact paths, ${tests.length} test candidates, and ${evidence.length} evidence references.` : "No canonical result is available for this operation yet.");
  const diagnostics = (result?.diagnostics ?? []).map((item) => typeof item === "string" ? item : item.message).filter(Boolean);
  const failed = value.lifecycle === "failed" || Boolean(value.error);
  const safeRisk = isSafeRisk(data.risk);
  const highRisk = isHighRisk(data.risk);
  const keyPoints = data.risk_assessment?.rationale?.filter(Boolean).slice(0, 7) ?? [];
  const fallbackPoints = [
    `${affectedFiles.length} affected file${affectedFiles.length === 1 ? "" : "s"} represented by returned evidence`,
    `${directCount} direct and ${indirectCount} indirect impact path${directCount + indirectCount === 1 ? "" : "s"} returned`,
    `${tests.length} test candidate${tests.length === 1 ? "" : "s"} returned by the canonical result`,
    `${evidence.length} evidence reference${evidence.length === 1 ? "" : "s"} available for review`,
  ];

  return (
    <main className="min-h-full w-full bg-background">
      <div className="mx-auto grid w-full max-w-[1500px] gap-4 p-4 sm:p-6 lg:p-8">
        <header className="flex flex-col gap-4 border-b border-border pb-4 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex min-w-0 items-start gap-3">
            <Link href="/" aria-label="Back to CodeMind" className="mt-1 grid size-9 shrink-0 place-items-center rounded-md border border-border bg-surface text-muted-foreground hover:text-foreground"><ArrowLeft className="size-4" /></Link>
            <div className="min-w-0"><p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Analysis workspace</p><h1 className="mt-1 truncate text-xl font-semibold sm:text-2xl">{value.answer ? value.answer.split("\n")[0].slice(0, 120) : "CodeMind analysis"}</h1><p className="mt-1 truncate font-mono text-xs text-muted-foreground">{analysisId}</p></div>
          </div>
          <div className="flex flex-wrap items-center gap-2"><OperationStatus status={value.lifecycle} outcome={result?.outcome} completeness={result?.completeness} />{context && <Badge variant="outline">{context.repository_id}</Badge>}</div>
        </header>

        {failed && <ErrorState title="Analysis failed" description={value.error?.message ?? "The operation did not produce a completed analysis."} />}
        {result && <CompletenessNotice completeness={result.completeness} limitations={[...result.limitations, ...(value.limitations ?? [])]} />}

        <Tabs value={activeTab} onValueChange={(tab) => tab && setActiveTab(tab)}>
          <TabsList aria-label="Analysis views" className="flex h-auto w-full gap-1 overflow-x-auto rounded-xl border border-border bg-surface p-1">
            {[
              ["chat", "Chat", BrainCircuit], ["execution", "Execution", Target], ["summary", "Summary", Sparkles], ["impact", "Impact Analysis", Network], ["files", `Affected Files (${affectedFiles.length})`, FileCode2], ["tests", `Test Impact (${tests.length})`, FlaskConical], ["graph", "Dependency Graph", Network], ["evidence", "Evidence", FileCode2], ["recommendations", "Recommendations", Check],
            ].map(([id, label, Icon]) => <TabsTrigger key={String(id)} value={String(id)} className="min-h-10 shrink-0 gap-1.5 rounded-lg border border-transparent px-3 text-xs data-active:border-primary data-active:bg-primary data-active:text-primary-foreground sm:text-sm"><Icon aria-hidden="true" className="size-3.5" />{label as string}</TabsTrigger>)}
          </TabsList>

          <TabsContent value="chat" className="pt-4"><Card><CardHeader><CardTitle className="flex items-center gap-2"><BrainCircuit className="size-5 text-accent" />Investigation response</CardTitle><CardDescription>The assistant response is shown separately from the canonical engine facts below.</CardDescription></CardHeader><CardContent className="grid gap-4"><p className="whitespace-pre-wrap text-sm leading-7">{summary}</p>{diagnostics.length > 0 && <div className="rounded-lg border border-border bg-surface-alt p-3"><p className="text-xs font-medium">Diagnostics</p>{diagnostics.slice(0, 20).map((item, index) => <p key={`${index}-${item}`} className="mt-1 text-xs text-muted-foreground">{item}</p>)}</div>}</CardContent></Card></TabsContent>

          <TabsContent value="execution" className="pt-4"><div className="grid gap-5 lg:grid-cols-[minmax(0,1.7fr)_minmax(320px,0.8fr)]"><ExecutionTimeline events={events} /><EngineAccordion events={events.map((event) => ({ ...event, engine: event.engine ?? "", stage: event.stage ?? event.status, stageState: event.stageState, progressSequence: event.progressSequence, progressEventId: event.progressEventId }))} /></div></TabsContent>

          <TabsContent value="summary" className="pt-4 grid gap-4">
            {safeRisk && <section className="rounded-xl border border-success/60 bg-success/5 p-5 sm:p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between"><div className="flex items-center gap-4"><span className="grid size-14 shrink-0 place-items-center rounded-full bg-success text-background"><Check className="size-7" /></span><div><h2 className="text-2xl font-semibold">No Risk Detected</h2><p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">The canonical risk assessment reports {riskLabel(data.risk).toLowerCase()}. Review the evidence and limitations below before making a change.</p></div></div><div className="rounded-lg border border-success/40 bg-background/30 px-5 py-3"><p className="text-xs text-muted-foreground">Risk Level</p><p className="mt-1 text-lg font-semibold text-success">{riskLabel(data.risk)}</p></div></div></section>}
            {!safeRisk && <section className={`rounded-xl border p-5 sm:p-6 ${highRisk ? "border-danger/60 bg-danger/5" : "border-border bg-surface"}`}><div className="flex items-center gap-3"><span className={`grid size-11 place-items-center rounded-full ${highRisk ? "bg-danger/10 text-danger" : "bg-warning/10 text-warning"}`}><ShieldCheck className="size-6" /></span><div><h2 className="text-xl font-semibold">Risk Assessment: {riskLabel(data.risk)}</h2><p className="mt-1 text-sm text-muted-foreground">The result did not qualify for the zero-risk presentation state.</p></div></div></section>}

            <div className="grid gap-4 lg:grid-cols-[minmax(0,1.6fr)_minmax(320px,0.8fr)]">
              <div className="grid gap-4">
                <SummaryPanel title="AI Summary" summary={summary} outcome={result?.outcome ?? "unavailable"} completeness={result?.completeness ?? "unknown"} freshness={result?.freshness ?? "unknown"} limitations={result?.limitations} />
                <Card><CardHeader><CardTitle className="flex items-center gap-2"><CheckCircle2 className="size-5 text-success" />Key Points</CardTitle><CardDescription>Only canonical rationale is shown when the engine supplied it; otherwise the remaining points are direct counts from this result.</CardDescription></CardHeader><CardContent><ul className="grid gap-2">{(keyPoints.length ? keyPoints : fallbackPoints).map((point, index) => <li key={`${index}-${point}`} className="flex gap-2 text-sm"><Check className="mt-0.5 size-4 shrink-0 text-success" /><span>{point}</span></li>)}</ul></CardContent></Card>
                <Card><CardHeader><CardTitle className="flex items-center gap-2"><FileCode2 className="size-5 text-primary" />What Was Analyzed?</CardTitle><CardDescription>CodeMind combines the supported intelligence dimensions returned for this investigation.</CardDescription></CardHeader><CardContent><div className="grid gap-3 md:grid-cols-3">{[{ name: "CoIE", title: "Code Intelligence Engine", description: "Code structure and dependencies." }, { name: "TIE", title: "Test Intelligence Engine", description: "Test coverage and test impact." }, { name: "ChIE", title: "Change Intelligence Engine", description: "Change impact and risk analysis." }].map((item) => <div key={item.name} className="rounded-lg border border-border bg-background/40 p-4"><div className="flex items-center justify-between gap-2"><span className="font-mono font-semibold text-accent">{item.name}</span><CheckCircle2 className="size-4 text-success" /></div><p className="mt-2 text-sm font-medium">{item.title}</p><p className="mt-1 text-xs leading-5 text-muted-foreground">{item.description}</p></div>)}</div></CardContent></Card>
                <Card><CardHeader><CardTitle className="flex items-center gap-2"><Check className="size-5 text-success" />Conclusion</CardTitle></CardHeader><CardContent className="grid gap-4"><p className="text-sm leading-7">{safeRisk ? "The canonical assessment indicates a low-risk or zero-risk change. You can proceed while keeping the returned evidence and limitations available for review." : highRisk ? "The canonical assessment indicates elevated risk. Review the affected paths, evidence and recommendations before proceeding." : "Review the canonical risk assessment, affected paths and evidence before proceeding with the change."}</p><div className="flex flex-wrap gap-2"><button type="button" onClick={() => setActiveTab("impact")} className="inline-flex min-h-10 items-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary-hover"><Network className="size-4" />View impact analysis</button><button type="button" onClick={() => setActiveTab("evidence")} className="inline-flex min-h-10 items-center gap-2 rounded-md border border-border px-4 text-sm hover:bg-surface-alt"><FileCode2 className="size-4" />View evidence</button></div></CardContent></Card>
              </div>

              <aside className="grid content-start gap-4">
                <RiskCard risk={data.risk ?? "unknown"} rationale={data.risk_assessment?.rationale} unknownFactors={data.risk_assessment?.unknown_factors} confidence={data.risk_assessment?.confidence !== undefined ? String(data.risk_assessment.confidence) : undefined} />
                <Card><CardHeader><CardTitle className="flex items-center gap-2"><Target className="size-5 text-accent" />Change Scope</CardTitle></CardHeader><CardContent className="grid gap-2">{[{ label: "Files Changed", value: affectedFiles.length, Icon: FileCode2 }, { label: "Direct Dependencies", value: directCount, Icon: Network }, { label: "Indirect Dependencies", value: indirectCount, Icon: Network }, { label: "Tests Affected", value: tests.length, Icon: FlaskConical }].map((item) => <div key={item.label} className="flex items-center justify-between gap-3 rounded-lg border border-border bg-background/40 px-3 py-2.5"><span className="flex items-center gap-2 text-sm"><item.Icon className="size-4 text-primary" />{item.label}</span><strong className="text-sm text-primary">{item.value}</strong></div>)}</CardContent></Card>
                <Card><CardHeader><CardTitle className="flex items-center gap-2"><Network className="size-5 text-accent" />Dependency Graph (Overview)</CardTitle></CardHeader><CardContent><MiniDependencyGraph edges={edges} /></CardContent></Card>
              </aside>
            </div>
          </TabsContent>

          <TabsContent value="impact" className="pt-4"><ImpactCard directCount={directCount} indirectCount={indirectCount} paths={(data.impact_paths ?? []).map((path, index) => ({ id: `${path.entity_id}-${index}`, label: path.entity_id, classification: path.classification, depth: path.depth }))} completeness={result?.completeness ?? "unknown"} /></TabsContent>
          <TabsContent value="files" className="pt-4"><AffectedFilesPanel files={affectedFiles} /></TabsContent>
          <TabsContent value="tests" className="pt-4"><TestImpactPanel tests={tests} availability="available" currentSnapshotId={context?.source_snapshot_id} /></TabsContent>
          <TabsContent value="graph" className="pt-4"><div className="grid gap-5 lg:grid-cols-[minmax(0,1.4fr)_minmax(320px,0.8fr)]"><DependencyGraph edges={edges} truncated={data.impact?.truncated || result?.completeness === "truncated"} /><GraphPathPanel paths={paths} completeness={result?.completeness ?? "unknown"} limitations={result?.limitations} /></div></TabsContent>
          <TabsContent value="evidence" className="pt-4"><EvidencePanel evidence={evidence} completeness={result?.completeness ?? "unknown"} limitations={result?.limitations} /></TabsContent>
          <TabsContent value="recommendations" className="pt-4"><RecommendationsPanel recommendations={recommendations} availability={data.recommendations ? "available" : "unavailable"} /></TabsContent>
        </Tabs>

        {context && <div className="grid gap-4 border-t border-border pt-5 lg:grid-cols-3"><MemoryInspector context={memoryContext} analysisId={analysisId} />{data.handoff && <HandoffStatus handoffId={data.handoff.handoff_id} status={data.handoff.status} targetStage={data.handoff.target_stage} sourceStage={data.handoff.source_stage} />}<Card><CardHeader><CardTitle>Analysis provenance</CardTitle><CardDescription>Identity needed to verify that evidence belongs to this repository snapshot and graph.</CardDescription></CardHeader><CardContent className="grid gap-2 text-xs"><Provenance label="Repository" value={context.repository_id} /><Provenance label="Snapshot" value={context.source_snapshot_id} /><Provenance label="Graph version" value={context.graph_version} /><button type="button" onClick={() => navigator.clipboard?.writeText(JSON.stringify(context))} className="mt-2 inline-flex min-h-9 items-center justify-center gap-2 rounded-md border border-border px-3 text-xs hover:bg-surface-alt"><ClipboardCopy className="size-3.5" />Copy provenance</button></CardContent></Card></div>}
      </div>
    </main>
  );
}

function Provenance({ label, value }: { label: string; value: string }) { return <div className="flex min-w-0 items-start justify-between gap-3 border-b border-border/60 py-2 last:border-0"><span className="text-muted-foreground">{label}</span><code className="max-w-[70%] break-all text-right font-mono">{value}</code></div>; }
