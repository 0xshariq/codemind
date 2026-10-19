import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function EmptyState({
  title,
  description,
  action,
  icon,
  className,
}: {
  title: string;
  description: string;
  action?: ReactNode;
  icon?: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("grid justify-items-center gap-3 rounded-xl border border-dashed border-border bg-surface/60 px-6 py-10 text-center", className)}>
      {icon && <div aria-hidden="true" className="text-muted-foreground">{icon}</div>}
      <h2 className="text-base font-semibold text-foreground">{title}</h2>
      <p className="max-w-lg text-sm leading-6 text-muted-foreground">{description}</p>
      {action}
    </section>
  );
}
