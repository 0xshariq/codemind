"use client";

import { Check, CircleAlert, CircleDot, Clock3, HelpCircle, LoaderCircle, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";

type StatusKind = "pending" | "running" | "completed" | "partial" | "failed" | "unavailable" | "unknown";

function normalize(value?: string): StatusKind {
  const status = value?.toLowerCase().replaceAll("_", " ") ?? "unknown";
  if (status.includes("unavailable")) return "unavailable";
  if (status.includes("fail")) return "failed";
  if (status.includes("partial") || status.includes("truncated") || status.includes("incomplete")) return "partial";
  if (status.includes("complete") || status === "success") return "completed";
  if (status.includes("running") || status.includes("planning") || status.includes("coie") || status.includes("tie") || status.includes("chie") || status.includes("reasoning")) return "running";
  if (status.includes("pending") || status.includes("idle") || status.includes("waiting")) return "pending";
  return "unknown";
}

const meta: Record<StatusKind, { label: string; icon: typeof Check; className: string }> = {
  pending: { label: "Waiting", icon: Clock3, className: "border-border text-muted-foreground" },
  running: { label: "In Progress", icon: LoaderCircle, className: "border-primary/40 bg-primary/5 text-primary" },
  completed: { label: "Completed", icon: Check, className: "border-success/40 bg-success/5 text-success" },
  partial: { label: "Partial", icon: CircleAlert, className: "border-warning/40 bg-warning/5 text-warning" },
  failed: { label: "Failed", icon: X, className: "border-danger/40 bg-danger/5 text-danger" },
  unavailable: { label: "Unavailable", icon: CircleAlert, className: "border-danger/40 bg-danger/5 text-danger" },
  unknown: { label: "Unknown", icon: HelpCircle, className: "border-border text-muted-foreground" },
};

export function OperationStatus({ status, outcome, completeness }: { status: string; outcome?: string; completeness?: string }) {
  const kind = normalize(status);
  const state = meta[kind];
  const Icon = state.icon;
  return <div className="flex flex-wrap items-center gap-2" aria-label={[state.label, outcome, completeness].filter(Boolean).join(", ")}>
    <span className={`inline-flex h-7 items-center gap-1.5 rounded-md border px-2 text-xs font-medium ${state.className}`}><Icon aria-hidden="true" className={`size-3.5 ${kind === "running" ? "animate-spin motion-reduce:animate-none" : ""}`} />{state.label}</span>
    {outcome && <Badge variant={normalize(outcome) === "completed" ? "default" : normalize(outcome) === "failed" || normalize(outcome) === "unavailable" ? "destructive" : "outline"}>Outcome: {outcome.replaceAll("_", " ")}</Badge>}
    {completeness && completeness !== "unknown" && <Badge variant={normalize(completeness) === "partial" ? "secondary" : "outline"}>Completeness: {completeness.replaceAll("_", " ")}</Badge>}
  </div>;
}
