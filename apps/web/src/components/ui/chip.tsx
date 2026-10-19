import type { HTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const chipVariants = cva("inline-flex w-fit items-center gap-1 rounded-full border px-2.5 py-1 text-xs font-medium leading-none", {
  variants: {
    variant: {
      default: "border-primary/30 bg-primary/10 text-primary",
      success: "border-success/30 bg-success/10 text-success",
      warning: "border-warning/30 bg-warning/10 text-warning",
      danger: "border-danger/30 bg-danger/10 text-danger",
      muted: "border-border bg-surface-alt text-muted-foreground",
    },
  },
  defaultVariants: { variant: "default" },
});

export function Chip({ className, variant, ...props }: HTMLAttributes<HTMLSpanElement> & VariantProps<typeof chipVariants>) {
  return <span className={cn(chipVariants({ variant }), className)} {...props} />;
}
