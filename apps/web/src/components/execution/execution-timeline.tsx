import {
  BrainCircuit,
  Check,
  CircleAlert,
  CircleDot,
  CircleMinus,
  Clock3,
  Code2,
  FlaskConical,
  LoaderCircle,
  Network,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type TimelineEvent = {
  sequence: number;
  status: string;
  message?: string;
  timestamp: string;
  engine?: string;
  stage?: string;
  stageState?: string;
  progressSequence?: number;
  progressEventId?: string;
};

type StageId = "planning" | "coie" | "tie" | "chie" | "final";
type StageStatus = "pending" | "running" | "completed" | "partial" | "skipped" | "failed";

type StageDefinition = {
  id: StageId;
  number: number;
  title: string;
  description: string;
  Icon: LucideIcon;
};

const STAGES: readonly StageDefinition[] = [
  {
    id: "planning",
    number: 1,
    title: "LLM (Planning & Tool Invocation)",
    description: "Understanding your request and planning the investigation...",
    Icon: BrainCircuit,
  },
  {
    id: "coie",
    number: 2,
    title: "CoIE (Code Intelligence Engine)",
    description: "Analyzing repository and building code graph...",
    Icon: Code2,
  },
  {
    id: "tie",
    number: 3,
    title: "TIE (Test Intelligence Engine)",
    description: "Analyzing test structure and building test graph...",
    Icon: FlaskConical,
  },
  {
    id: "chie",
    number: 4,
    title: "ChIE (Change Intelligence Engine)",
    description: "Analyzing change impact and building change graph...",
    Icon: Network,
  },
  {
    id: "final",
    number: 5,
    title: "LLM (Reasoning & Generating Response)",
    description: "Synthesizing results from all engines and generating insights...",
    Icon: BrainCircuit,
  },
];

function normalized(value?: string) {
  return value?.trim().toLowerCase() ?? "";
}

function eventStage(event: TimelineEvent): StageId | null {
  const status = normalized(event.status);
  const engineStage = `${normalized(event.engine)} ${normalized(event.stage)}`;

  if (status === "planning" || status === "idle" || /\bplanning\b/.test(engineStage)) return "planning";
  if (status === "coie" || engineStage.includes("coie")) return "coie";
  if (status === "tie" || engineStage.includes("tie")) return "tie";
  if (status === "chie" || engineStage.includes("chie")) return "chie";
  if (status === "reasoning" || status === "completed" || status === "partial" || status === "failed") return "final";

  return null;
}

function eventState(event: TimelineEvent): StageStatus | null {
  const stageState = normalized(event.stageState);
  if (stageState === "failed") return "failed";
  if (stageState === "running") return "running";
  if (stageState === "completed") return "completed";
  if (stageState === "skipped") return "skipped";

  const status = normalized(event.status);
  if (status === "failed") return "failed";
  if (status === "reasoning") return "running";
  if (status === "completed") return "completed";
  if (status === "partial") return "partial";

  return null;
}

function uniqueOrderedEvents(events: readonly TimelineEvent[]) {
  const sorted = [...events].sort((left, right) => left.sequence - right.sequence);
  const seen = new Set<string>();

  return sorted.filter((item) => {
    const key = item.progressEventId
      ? `progress:${item.progressEventId}`
      : `sequence:${item.sequence}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function stageStatus(stage: StageId, events: readonly TimelineEvent[]): StageStatus {
  const stageIndex = STAGES.findIndex((item) => item.id === stage);
  const stageEvents = events.filter((event) => eventStage(event) === stage);
  if (!stageEvents.length) return "pending";

  if (stageEvents.some((event) => eventState(event) === "failed")) return "failed";

  const latest = stageEvents.at(-1);
  const latestState = latest ? eventState(latest) : null;
  if (latestState === "completed" || latestState === "skipped" || latestState === "partial") return latestState;

  const hasLaterStage = events.some((event) => {
    const eventStageId = eventStage(event);
    if (!eventStageId) return false;
    return STAGES.findIndex((item) => item.id === eventStageId) > stageIndex
      && event.sequence > (latest?.sequence ?? -1);
  });
  if (hasLaterStage) return "completed";

  return latestState === "failed" ? "failed" : "running";
}

function latestStageMessage(stage: StageId, events: readonly TimelineEvent[], fallback: string) {
  const latest = [...events].reverse().find((event) => event.message?.trim());
  return latest?.message?.trim() || fallback;
}

function formatTimestamp(timestamp: string) {
  if (!timestamp) return "";
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(parsed);
}

function formatDuration(events: readonly TimelineEvent[]) {
  if (events.length < 2) return "";
  const first = Date.parse(events[0]?.timestamp ?? "");
  const last = Date.parse(events.at(-1)?.timestamp ?? "");
  const delta = last - first;
  if (!Number.isFinite(delta) || delta <= 0) return "";
  return `${(delta / 1000).toFixed(1)}s`;
}

function statusLabel(status: StageStatus) {
  if (status === "completed") return "Completed";
  if (status === "partial") return "Partial";
  if (status === "skipped") return "Skipped";
  if (status === "running") return "In Progress";
  if (status === "failed") return "Failed";
  return "Waiting";
}

function StageStatusIcon({ status }: { status: StageStatus }) {
  if (status === "completed") return <Check aria-hidden="true" className="size-4" />;
  if (status === "partial" || status === "failed") return <CircleAlert aria-hidden="true" className="size-4" />;
  if (status === "skipped") return <CircleMinus aria-hidden="true" className="size-4" />;
  if (status === "running") return <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />;
  return <Clock3 aria-hidden="true" className="size-4" />;
}

function stageTone(status: StageStatus) {
  if (status === "completed") return "text-success";
  if (status === "partial") return "text-warning";
  if (status === "running") return "text-primary";
  if (status === "failed") return "text-destructive";
  return "text-muted-foreground";
}

function stageIconTone(status: StageStatus) {
  if (status === "running") return "border-primary/30 bg-primary/10 text-primary";
  if (status === "completed") return "border-success/30 bg-success/10 text-success";
  if (status === "partial") return "border-warning/30 bg-warning/10 text-warning";
  if (status === "failed") return "border-destructive/30 bg-destructive/10 text-destructive";
  return "border-border bg-surface-alt text-muted-foreground";
}

function EventRow({ event, eventKey }: { event: TimelineEvent; eventKey: string | number }) {
  const state = eventState(event) ?? "running";
  const label = event.message?.trim() || event.stage?.trim() || event.status.replaceAll("_", " ");
  const timestamp = formatTimestamp(event.timestamp);

  return (
    <li key={eventKey} className="grid grid-cols-[1.5rem_minmax(0,1fr)_auto] items-center gap-3 border-t border-border/60 py-2.5 first:border-t-0">
      <span aria-hidden="true" className={`grid size-5 place-items-center rounded-full border ${stageIconTone(state)}`}>
        <StageStatusIcon status={state} />
      </span>
      <span className="min-w-0 text-sm text-foreground">{label}</span>
      <span className="whitespace-nowrap text-xs text-muted-foreground">{timestamp || statusLabel(state)}</span>
    </li>
  );
}

function ResponsePreparationCard() {
  return (
    <section className="rounded-xl border border-border bg-surface p-4 sm:p-5" aria-label="Response preparation">
      <div className="flex items-start gap-3">
        <span className="grid size-10 shrink-0 place-items-center rounded-xl border border-primary/40 bg-primary/10 text-primary">
          <BrainCircuit aria-hidden="true" className="size-5" />
        </span>
        <div className="min-w-0">
          <h2 className="text-base font-semibold">CodeMind is preparing your analysis...</h2>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">The engines have completed their work. CodeMind is preparing the response from the returned analysis facts, evidence and recommendations.</p>
        </div>
      </div>
      <div className="mt-4 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
        {[
          ["Summarizing impact", "Preparing affected components and paths."],
          ["Calculating risk factors", "Preparing the canonical risk assessment."],
          ["Collecting test coverage", "Preparing supported test-impact evidence."],
          ["Preparing evidence", "Structuring graph and source references."],
        ].map(([title, description]) => (
          <div key={title} className="rounded-lg border border-border bg-background/50 p-3">
            <p className="text-sm font-medium">{title}</p>
            <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function StageCard({ stage, status, events }: { stage: StageDefinition; status: StageStatus; events: readonly TimelineEvent[] }) {
  const isOpen = status === "running" || status === "partial" || status === "failed";
  const StageIcon = stage.Icon;
  const duration = formatDuration(events);
  const isFinal = stage.id === "final";

  return (
    <article
      data-stage={stage.id}
      data-stage-status={status}
      className={`overflow-hidden rounded-xl border bg-surface transition-colors ${
        status === "running"
          ? "border-primary"
          : status === "partial"
            ? "border-warning/60"
            : status === "failed"
              ? "border-destructive/60"
              : "border-border"
      }`}
    >
      <div className="grid grid-cols-[3rem_minmax(0,1fr)_auto] items-center gap-3 px-4 py-4">
        <span className={`grid size-12 place-items-center rounded-xl border ${stageIconTone(status)}`}>
          <StageIcon aria-hidden="true" className="size-6" />
        </span>
        <div className="min-w-0">
          <h3 className="text-base font-semibold leading-6">{stage.number}. {stage.title}</h3>
          <p className="mt-0.5 truncate text-sm text-muted-foreground">{latestStageMessage(stage.id, events, stage.description)}</p>
        </div>
        <div className={`flex items-center gap-2 text-xs ${stageTone(status)}`}>
          <span className="hidden rounded-md border border-current/20 bg-current/5 px-2 py-1 sm:inline">{statusLabel(status)}</span>
          {duration && <span className="hidden text-muted-foreground md:inline">{duration}</span>}
          <span aria-label={statusLabel(status)} className="grid size-7 place-items-center"><StageStatusIcon status={status} /></span>
        </div>
      </div>

      {isOpen && (
        <div className="border-t border-border/60 px-4 pb-3">
          {isFinal && <div className="pt-3"><div className="h-2 overflow-hidden rounded-full bg-surface-alt"><div className="h-full w-2/5 animate-pulse rounded-full bg-primary motion-reduce:animate-none" /></div><p className="mt-2 text-right text-xs text-muted-foreground">Generating response</p></div>}
          {events.length ? (
            <ol aria-label={`${stage.title} events`} className="pt-2">
              {events.map((event) => <EventRow key={event.progressEventId ?? event.sequence} eventKey={event.progressEventId ?? event.sequence} event={event} />)}
            </ol>
          ) : (
            <p className="py-4 text-sm text-muted-foreground">{status === "failed" ? "The stage failed before a detailed progress event was returned." : "The stage is running; detailed progress has not been returned yet."}</p>
          )}
        </div>
      )}
    </article>
  );
}

export function ExecutionTimeline({ events }: { events: readonly TimelineEvent[] }) {
  const ordered = uniqueOrderedEvents(events);
  const eventsByStage = new Map<StageId, TimelineEvent[]>();

  for (const stage of STAGES) eventsByStage.set(stage.id, []);
  for (const event of ordered) {
    const stage = eventStage(event);
    if (stage) eventsByStage.get(stage)?.push(event);
  }

  const hasEvents = ordered.length > 0;
  const finalStatus = stageStatus("final", ordered);

  return (
    <section aria-label="Investigation execution" className="grid gap-4">
      <ol className="relative grid gap-3" aria-label="Investigation stages">
        {STAGES.map((stage, index) => {
          const stageEvents = eventsByStage.get(stage.id) ?? [];
          const status = stageStatus(stage.id, ordered);

          return (
            <li key={stage.id} className="relative pl-0 sm:pl-7">
              {index < STAGES.length - 1 && (
                <span aria-hidden="true" className={`absolute left-2 top-14 hidden h-[calc(100%+0.75rem)] w-px sm:block ${status === "completed" ? "bg-success/70" : status === "running" ? "bg-primary/60" : "bg-border"}`} />
              )}
              <span aria-hidden="true" className={`absolute left-0 top-5 hidden size-4 -translate-x-1/2 rounded-full border-2 sm:block ${status === "completed" ? "border-success bg-success" : status === "running" ? "border-primary bg-primary" : status === "failed" ? "border-destructive bg-destructive" : "border-border bg-background"}`} />
              <StageCard stage={stage} status={status} events={stageEvents} />
            </li>
          );
        })}
      </ol>

      {finalStatus === "running" && <ResponsePreparationCard />}

      {!hasEvents && (
        <div className="flex items-center gap-2 rounded-xl border border-dashed border-border px-4 py-5 text-sm text-muted-foreground">
          <CircleDot aria-hidden="true" className="size-4 shrink-0" />
          <p>No operation events have been returned yet. Stages will update from the live operation stream.</p>
        </div>
      )}
    </section>
  );
}
