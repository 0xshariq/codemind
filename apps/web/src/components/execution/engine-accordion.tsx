import { ChevronDown, Cpu } from "lucide-react";

export type EngineProgress = { sequence: number; engine: string; stage: string; stageState?: string; message: string; timestamp: string; progressSequence?: number };

export function EngineAccordion({ events }: { events: readonly EngineProgress[] }) {
  const groups = ["coie", "tie", "chie"].map((engine) => ({
    engine,
    events: events.filter((event) => `${event.engine} ${event.stage}`.toLowerCase().includes(engine)).sort((a, b) => a.sequence - b.sequence),
  }));
  return <section className="grid gap-3" aria-label="Engine progress"><h2 className="text-base font-semibold">Engine stages</h2><div className="grid gap-2">{groups.map(({ engine, events: stageEvents }) => {
    const latest = stageEvents.at(-1);
    const label = engine === "coie" ? "Code intelligence" : engine === "tie" ? "Test intelligence" : "Change intelligence";
    return <details key={engine} className="group rounded-lg border border-border bg-surface"><summary className="flex min-h-12 cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><span className="flex items-center gap-2"><Cpu aria-hidden="true" className="size-4 text-primary" /><span className="text-sm font-medium">{label}</span></span><span className="flex items-center gap-2 text-xs text-muted-foreground">{latest ? latest.stageState ?? latest.stage.replaceAll("_", " ") : "Waiting for engine"}<ChevronDown aria-hidden="true" className="size-4 transition-transform group-open:rotate-180" /></span></summary><div className="grid gap-2 border-t border-border px-4 py-3">{stageEvents.length ? stageEvents.map((event) => <div key={event.sequence} className="grid gap-1"><p className="text-sm">{event.message || event.stage}</p><time dateTime={event.timestamp} className="font-mono text-xs text-muted-foreground">{event.stageState ?? "event"} · engine event #{event.progressSequence ?? event.sequence} · {event.timestamp}</time></div>) : <p className="text-sm text-muted-foreground">No Rust progress event has been returned for this engine yet.</p>}</div></details>;
  })}</div></section>;
}
