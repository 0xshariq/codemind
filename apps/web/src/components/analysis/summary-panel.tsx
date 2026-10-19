import { CompletenessNotice } from "@/components/common/completeness-notice";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function SummaryPanel({
  title = "Analysis summary",
  summary,
  outcome,
  completeness,
  freshness,
  limitations = [],
}: {
  title?: string;
  summary?: string;
  outcome: string;
  completeness: string;
  freshness: string;
  limitations?: readonly string[];
}) {
  return <Card className="border-border bg-card">
    <CardHeader><div className="flex flex-wrap items-center justify-between gap-3"><CardTitle>{title}</CardTitle><div className="flex flex-wrap gap-2"><Badge variant="outline">{outcome.replaceAll("_", " ")}</Badge><Badge variant="secondary">{freshness.replaceAll("_", " ")}</Badge></div></div><CardDescription>Structured status from the canonical analysis result.</CardDescription></CardHeader>
    <CardContent className="grid gap-4">{summary && <p className="whitespace-pre-wrap text-sm leading-6">{summary}</p>}<CompletenessNotice completeness={completeness} limitations={limitations} /></CardContent>
  </Card>;
}
