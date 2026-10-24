import { CircleAlert, ShieldCheck, ShieldQuestion } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function RiskCard({ risk, rationale = [], unknownFactors = [], confidence, reviewSeverity }: { risk: string; rationale?: readonly string[]; unknownFactors?: readonly string[]; confidence?: string; reviewSeverity?: string }) {
  const normalized = risk.toLowerCase();
  const safe = normalized.includes("none") || normalized.includes("low") || normalized.includes("zero");
  const high = normalized.includes("high") || normalized.includes("critical");
  const Icon = safe ? ShieldCheck : high ? CircleAlert : ShieldQuestion;
  const tone = safe ? "text-success" : high ? "text-danger" : "text-warning";
  return <Card className="border-border bg-surface"><CardHeader><CardTitle className="flex items-center gap-2"><Icon aria-hidden="true" className={`size-5 ${tone}`} />Risk Assessment</CardTitle><CardDescription>Risk is a canonical assessment dimension and is not inferred from impact counts.</CardDescription></CardHeader><CardContent className="grid gap-4"><div className="flex flex-wrap items-center gap-2"><span className={`text-xl font-semibold capitalize ${tone}`}>{risk.replaceAll("_", " ")}</span>{confidence && <Badge variant="outline">Confidence {confidence}</Badge>}{reviewSeverity && <Badge variant="outline">Review {reviewSeverity.replaceAll("_", " ")}</Badge>}</div>{rationale.length ? <ul className="grid gap-2">{rationale.slice(0, 30).map((item, index) => <li key={`${index}-${item}`} className="flex gap-2 text-sm"><span className="mt-1 size-1.5 shrink-0 rounded-full bg-current" /><span>{item}</span></li>)}</ul> : <p className="text-sm text-muted-foreground">No risk rationale was supplied by ChIE.</p>}{unknownFactors.length > 0 && <div className="rounded-lg border border-warning/30 bg-warning/5 p-3"><p className="text-xs font-medium text-warning">Unknown factors</p><ul className="mt-2 grid gap-1 text-sm text-muted-foreground">{unknownFactors.slice(0, 20).map((item, index) => <li key={`${index}-${item}`}>{item}</li>)}</ul></div>}</CardContent></Card>;
}
