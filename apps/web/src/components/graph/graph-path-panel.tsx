import { ArrowRight } from "lucide-react";
import { CompletenessNotice } from "@/components/common/completeness-notice";

export type GraphPath = {
  id: string;
  classification: string;
  entities: readonly { id: string; label: string }[];
  relationships: readonly { kind: string; from: string; to: string }[];
};

export function GraphPathPanel({
  paths,
  completeness,
  limitations = [],
}: {
  paths: readonly GraphPath[];
  completeness: string;
  limitations?: readonly string[];
}) {
  return <section className="grid gap-4" aria-label="Evidence paths"><div><h2 className="text-base font-semibold">How this result was reached</h2><p className="mt-1 text-sm text-muted-foreground">Relationship paths are shown as returned by the canonical graph analysis.</p></div>
    <CompletenessNotice completeness={completeness} limitations={limitations} />
    {paths.length === 0 ? <p className="rounded-lg border border-dashed border-border p-5 text-center text-sm text-muted-foreground">No explanatory paths were returned.</p> : <ol className="grid gap-3">{paths.slice(0, 200).map((path) => <li key={path.id} className="grid gap-3 rounded-lg border border-border bg-surface p-4"><p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{path.classification.replaceAll("_", " ")}</p><ol className="grid gap-2">{path.relationships.slice(0, 100).map((relationship, index) => { const from = path.entities.find((entity) => entity.id === relationship.from)?.label ?? relationship.from; const to = path.entities.find((entity) => entity.id === relationship.to)?.label ?? relationship.to; return <li key={`${relationship.from}-${relationship.kind}-${relationship.to}-${index}`} className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2"><span className="min-w-0"><code className="block break-all font-mono text-xs">{from}</code></span><span className="grid justify-items-center gap-1 text-[10px] text-muted-foreground"><span>{relationship.kind.replaceAll("_", " ")}</span><ArrowRight aria-hidden="true" className="size-4 text-primary" /></span><span className="min-w-0"><code className="block break-all font-mono text-xs">{to}</code></span></li>; })}</ol></li>)}</ol>}
  </section>;
}
