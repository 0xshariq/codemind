import { Check, CircleHelp } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { Badge } from "@/components/ui/badge";

export type Recommendation = { id: string; title: string; detail?: string; priority?: string; evidence?: readonly string[] };

export function RecommendationsPanel({
  recommendations,
  availability = "available",
}: {
  recommendations: readonly Recommendation[];
  availability?: "available" | "unsupported" | "unavailable";
}) {
  if (availability !== "available") return <EmptyState title={availability === "unsupported" ? "Recommendations unsupported" : "Recommendations unavailable"} description="The engine did not return recommendation data for this result." icon={<CircleHelp className="size-6" />} />;
  const unique = [...new Map(recommendations.map((item) => [item.id, item])).values()].slice(0, 100);
  if (unique.length === 0) return <EmptyState title="No recommendations returned" description="No recommended follow-up actions or tests were supported by the available evidence." />;
  return <section className="grid gap-3" aria-label="Recommendations"><ul className="grid gap-2">{unique.map((item) => <li key={item.id} className="grid gap-2 rounded-lg border border-border bg-surface p-3"><div className="flex flex-wrap items-center gap-2"><Check aria-hidden="true" className="size-4 text-primary" /><span className="text-sm font-medium">{item.title}</span>{item.priority && <Badge variant="outline">{item.priority.replaceAll("_", " ")}</Badge>}</div>{item.detail && <p className="text-sm text-muted-foreground">{item.detail}</p>}{item.evidence?.length ? <ul className="grid gap-1 border-t border-border pt-2">{item.evidence.slice(0, 20).map((source, index) => <li key={`${index}-${source}`} className="break-all font-mono text-xs text-muted-foreground">Evidence: {source}</li>)}</ul> : <p className="text-xs text-muted-foreground">No supporting evidence reference was supplied.</p>}</li>)}</ul></section>;
}
