import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";

export function IconButton({
  label,
  children,
  className,
  variant = "ghost",
  disabled,
  onClick,
  type = "button",
}: {
  label: string;
  children: ReactNode;
  className?: string;
  variant?: "default" | "outline" | "secondary" | "ghost" | "destructive" | "link";
  disabled?: boolean;
  onClick?: () => void;
  type?: "button" | "submit" | "reset";
}) {
  return <Button type={type} variant={variant} size="icon" aria-label={label} title={label} disabled={disabled} onClick={onClick} className={className}>{children}</Button>;
}
