import { ShieldAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function RiskCard({
  risk,
  rationale = [],
  unknownFactors = [],
  confidence,
  reviewSeverity,
}: {
  risk: string;
  rationale?: readonly string[];
  unknownFactors?: readonly string[];
  confidence?: string;
  reviewSeverity?: string;
}) {
  const label = risk.replaceAll("_", " ");
  return <Card className="border-border bg-card">
    <CardHeader><CardTitle className="flex items-center gap-2"><ShieldAlert aria-hidden="true" className="size-5 text-warning" />Risk assessment</CardTitle><CardDescription>Qualitative assessment from the canonical result; impact and evidence completeness remain separate.</CardDescription></CardHeader>
    <CardContent className="grid gap-3"><div className="flex flex-wrap gap-2"><Badge variant="secondary">Risk: {label}</Badge>{confidence && <Badge variant="outline">Confidence: {confidence.replaceAll("_", " ")}</Badge>}{reviewSeverity && <Badge variant="outline">Review: {reviewSeverity.replaceAll("_", " ")}</Badge>}</div>
      {rationale.length ? <ul className="grid list-disc gap-1 pl-5 text-sm">{rationale.slice(0, 30).map((item, index) => <li key={`${index}-${item}`}>{item}</li>)}</ul> : <p className="text-sm text-muted-foreground">No risk rationale was returned.</p>}
      {unknownFactors.length > 0 && <div className="grid gap-1 border-t border-border pt-3"><h3 className="text-xs font-medium text-muted-foreground">Unknown factors</h3>{unknownFactors.slice(0, 20).map((item, index) => <p key={`${index}-${item}`} className="text-sm text-muted-foreground">{item}</p>)}</div>}
    </CardContent>
  </Card>;
}
