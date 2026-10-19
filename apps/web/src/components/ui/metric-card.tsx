import type { ReactNode } from "react";

export function MetricCard({
  label,
  value,
  description,
  icon,
  tone = "default",
}: {
  label: string;
  value: ReactNode;
  description?: string;
  icon?: ReactNode;
  tone?: "default" | "success" | "warning" | "danger";
}) {
  const iconTone = { default: "text-primary", success: "text-success", warning: "text-warning", danger: "text-danger" }[tone];
  return <section className="grid gap-3 rounded-xl border border-border bg-surface p-4" aria-label={label}>
    <div className="flex items-center justify-between gap-3"><h3 className="text-sm text-muted-foreground">{label}</h3>{icon && <span aria-hidden="true" className={iconTone}>{icon}</span>}</div>
    <p className="text-2xl font-semibold tracking-tight text-foreground">{value}</p>
    {description && <p className="text-xs leading-5 text-muted-foreground">{description}</p>}
  </section>;
}
