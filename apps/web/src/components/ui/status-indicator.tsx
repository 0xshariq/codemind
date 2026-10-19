import { cn } from "@/lib/utils";

const tones = {
  success: "bg-success",
  warning: "bg-warning",
  danger: "bg-danger",
  info: "bg-primary",
  neutral: "bg-muted-foreground",
} as const;

export function StatusIndicator({
  label,
  tone = "neutral",
  className,
}: {
  label: string;
  tone?: keyof typeof tones;
  className?: string;
}) {
  return <span className={cn("inline-flex items-center gap-2 text-sm", className)}>
    <span aria-hidden="true" className={cn("size-2 shrink-0 rounded-full", tones[tone])} />
    <span>{label}</span>
  </span>;
}
