"use client";

import { useMemo, useState } from "react";
import { Check, ChevronDown, CircleAlert, CircleDot, Clock3, Code2, FlaskConical, Network } from "lucide-react";

export type EngineProgress = {
  sequence: number;
  engine: string;
  stage: string;
  stageState?: string;
  message: string;
  timestamp: string;
  progressSequence?: number;
  progressEventId?: string;
};

const ENGINES = [
  { id: "coie", number: 2, title: "CoIE", description: "Code Intelligence Engine", icon: Code2, tone: "text-primary" },
  { id: "tie", number: 3, title: "TIE", description: "Test Intelligence Engine", icon: FlaskConical, tone: "text-accent" },
  { id: "chie", number: 4, title: "ChIE", description: "Change Intelligence Engine", icon: Network, tone: "text-success" },
] as const;

function stateOf(event?: EngineProgress) {
  const state = event?.stageState?.toLowerCase();
  if (state === "failed") return "failed";
  if (state === "completed" || state === "skipped") return state;
  if (state === "running" || state === "pending") return state;
  if (event?.message && /failed|error/i.test(event.message)) return "failed";
  return event ? "running" : "pending";
}

function formatDuration(start?: string, end?: string) {
  if (!start || !end) return "";
  const delta = Date.parse(end) - Date.parse(start);
  if (!Number.isFinite(delta) || delta < 0) return "";
  return `${(delta / 1000).toFixed(1)}s`;
}

export function EngineAccordion({ events }: { events: readonly EngineProgress[] }) {
  const [open, setOpen] = useState<string[]>(["coie"]);
  const groups = useMemo(() => ENGINES.map((engine) => {
    const items = events
      .filter((event) => `${event.engine} ${event.stage}`.toLowerCase().includes(engine.id))
      .sort((a, b) => a.sequence - b.sequence);
    const latest = items.at(-1);
    const first = items[0];
    return { ...engine, items, latest, state: stateOf(latest), duration: formatDuration(first?.timestamp, latest?.timestamp) };
  }), [events]);

  function toggle(id: string) {
    setOpen((current) => current.includes(id) ? current.filter((value) => value !== id) : [...current, id]);
  }

  return (
    <section className="grid gap-3" aria-label="Engine pipeline stages">
      <div>
        <h2 className="text-base font-semibold">Engine stages</h2>
        <p className="mt-1 text-xs text-muted-foreground">Detailed Rust engine progress. LLM planning and final reasoning remain separate pipeline stages.</p>
      </div>
      <div className="grid gap-2">
        {groups.map((group) => {
          const isOpen = open.includes(group.id);
          const Icon = group.icon;
          const StateIcon = group.state === "completed" ? Check : group.state === "failed" ? CircleAlert : group.state === "running" ? CircleDot : Clock3;
          const stateLabel = group.state === "completed" ? "Completed" : group.state === "failed" ? "Failed" : group.state === "running" ? "In Progress" : "Waiting";
          return (
            <article key={group.id} className={`overflow-hidden rounded-xl border bg-surface ${group.state === "running" ? "border-primary" : group.state === "failed" ? "border-danger/60" : "border-border"}`}>
              <button type="button" aria-expanded={isOpen} onClick={() => toggle(group.id)} className="flex min-h-20 w-full items-center gap-3 px-4 py-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset">
                <span className={`grid size-11 shrink-0 place-items-center rounded-lg bg-surface-alt ${group.tone}`}><Icon aria-hidden="true" className="size-5" /></span>
                <span className="min-w-0 flex-1"><span className="block text-sm font-semibold">{group.number}. {group.title} <span className="font-normal text-muted-foreground">({group.description})</span></span><span className="mt-1 block truncate text-xs text-muted-foreground">{group.latest?.message ?? `Waiting for ${group.description} progress...`}</span></span>
                <span className="hidden shrink-0 items-center gap-2 sm:flex"><span className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs ${group.state === "completed" ? "border-success/40 bg-success/5 text-success" : group.state === "failed" ? "border-danger/40 bg-danger/5 text-danger" : group.state === "running" ? "border-primary/40 bg-primary/5 text-primary" : "border-border text-muted-foreground"}`}><StateIcon aria-hidden="true" className="size-3.5" />{stateLabel}</span>{group.duration && <span className="text-xs text-muted-foreground">{group.duration}</span>}</span>
                <ChevronDown aria-hidden="true" className={`size-4 shrink-0 text-muted-foreground transition-transform ${isOpen ? "rotate-180" : ""}`} />
              </button>
              {isOpen && <div className="border-t border-border px-4 py-3">
                {group.items.length === 0 ? <p className="rounded-lg border border-dashed border-border p-4 text-sm text-muted-foreground">No Rust progress event has been returned for this engine yet.</p> : <ol className="grid gap-0">{group.items.map((item, index) => {
                  const state = stateOf(item);
                  const IconForItem = state === "completed" ? Check : state === "failed" ? CircleAlert : state === "running" ? CircleDot : Clock3;
                  return <li key={item.progressEventId ?? `${item.sequence}-${index}`} className="flex min-w-0 items-start gap-3 border-b border-border/60 py-2 last:border-b-0"><span className={`mt-0.5 grid size-5 shrink-0 place-items-center rounded-full ${state === "completed" ? "bg-success text-background" : state === "failed" ? "bg-danger text-background" : state === "running" ? "border-2 border-primary text-primary" : "border-2 border-border text-muted-foreground"}`}><IconForItem aria-hidden="true" className="size-3" /></span><span className="min-w-0 flex-1 text-sm">{item.message || item.stage}<span className="mt-0.5 block text-[11px] text-muted-foreground">{item.stage.replaceAll("_", " ")}{item.progressSequence !== undefined ? ` · event ${item.progressSequence}` : ""}</span></span><time dateTime={item.timestamp} className="shrink-0 text-xs text-muted-foreground">{new Date(item.timestamp).toLocaleTimeString([], { minute: "2-digit", second: "2-digit" })}</time></li>;
                })}</ol>}
              </div>}
            </article>
          );
        })}
      </div>
    </section>
  );
}
