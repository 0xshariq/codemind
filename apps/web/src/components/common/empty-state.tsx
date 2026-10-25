import type { ReactNode } from "react";
import { Inbox } from "lucide-react";
import { cn } from "@/lib/utils";
export function EmptyState({ title, description, action, icon, className }: { title: string; description: string; action?: ReactNode; icon?: ReactNode; className?: string }) { return <section className={cn("grid justify-items-center gap-3 rounded-xl border border-dashed border-border bg-surface px-6 py-10 text-center", className)} aria-live="polite"><div aria-hidden="true" className="grid size-10 place-items-center rounded-lg bg-surface-alt text-muted-foreground">{icon ?? <Inbox className="size-5" />}</div><h2 className="text-base font-semibold">{title}</h2><p className="max-w-lg text-sm leading-6 text-muted-foreground">{description}</p>{action}</section>; }
