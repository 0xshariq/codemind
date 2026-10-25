import { BrainCircuit, CheckCircle2 } from "lucide-react";
import { CompletenessNotice } from "@/components/common/completeness-notice";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function SummaryPanel({ title = "AI Summary", summary, outcome, completeness, freshness, limitations = [] }: { title?: string; summary?: string; outcome: string; completeness: string; freshness: string; limitations?: readonly string[] }) {
  return <Card className="border-border bg-surface">
    <CardHeader><div className="flex items-start justify-between gap-3"><div className="flex items-center gap-3"><span className="grid size-10 place-items-center rounded-lg bg-accent/10 text-accent"><BrainCircuit aria-hidden="true" className="size-5" /></span><div><CardTitle>{title}</CardTitle><CardDescription className="mt-1">Response generated from the investigation context.</CardDescription></div></div><div className="flex flex-wrap gap-2"><Badge variant="outline">{outcome.replaceAll("_", " ")}</Badge><Badge variant="outline">{freshness.replaceAll("_", " ")}</Badge></div></div></CardHeader>
    <CardContent className="grid gap-4"><p className="whitespace-pre-wrap text-sm leading-7">{summary?.trim() || "No explanatory response was persisted for this result."}</p><div className="flex items-center gap-2 text-xs text-muted-foreground"><CheckCircle2 aria-hidden="true" className="size-4 text-success" />Canonical engine facts remain separate from this explanation.</div><CompletenessNotice completeness={completeness} limitations={limitations} /></CardContent>
  </Card>;
}
