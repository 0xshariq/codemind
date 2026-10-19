import { FileCode2 } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { CompletenessNotice } from "@/components/common/completeness-notice";

export type EvidenceItem = {
  id: string;
  sourceEntity?: string;
  targetEntity?: string;
  relationship?: string;
  path?: string;
  startLine?: number;
  endLine?: number;
  snapshotId?: string;
  graphVersion?: string;
  diagnostic?: string;
};

export function EvidencePanel({
  evidence,
  completeness = "unknown",
  limitations = [],
}: {
  evidence: readonly EvidenceItem[];
  completeness?: string;
  limitations?: readonly string[];
}) {
  return <section className="grid gap-4" aria-label="Evidence">
    <div><h2 className="text-base font-semibold">Evidence</h2><p className="mt-1 text-sm text-muted-foreground">Source references and graph derivations retain their provenance.</p></div>
    <CompletenessNotice completeness={completeness} limitations={limitations} />
    {evidence.length === 0 ? <EmptyState title="No evidence references" description="The canonical result did not provide source or relationship evidence." /> : <ul className="grid gap-2">{evidence.slice(0, 500).map((item) => <li key={item.id} className="grid gap-2 rounded-lg border border-border bg-surface p-3"><div className="flex items-center gap-2 text-xs font-medium"><FileCode2 aria-hidden="true" className="size-4 text-primary" />Evidence record</div>{item.path && <code className="break-all font-mono text-xs">{item.path}{item.startLine !== undefined ? `:${item.startLine}${item.endLine !== undefined && item.endLine !== item.startLine ? `-${item.endLine}` : ""}` : ""}</code>}<dl className="grid gap-1 text-xs sm:grid-cols-2">{item.sourceEntity && <div><dt className="inline text-muted-foreground">Source: </dt><dd className="inline break-all font-mono">{item.sourceEntity}</dd></div>}{item.targetEntity && <div><dt className="inline text-muted-foreground">Target: </dt><dd className="inline break-all font-mono">{item.targetEntity}</dd></div>}{item.relationship && <div><dt className="inline text-muted-foreground">Relationship: </dt><dd className="inline">{item.relationship.replaceAll("_", " ")}</dd></div>}{item.snapshotId && <div><dt className="inline text-muted-foreground">Snapshot: </dt><dd className="inline break-all font-mono">{item.snapshotId}</dd></div>}{item.graphVersion && <div><dt className="inline text-muted-foreground">Graph: </dt><dd className="inline break-all font-mono">{item.graphVersion}</dd></div>}</dl>{item.diagnostic && <p className="text-xs text-warning">{item.diagnostic}</p>}</li>)}</ul>}
  </section>;
}
