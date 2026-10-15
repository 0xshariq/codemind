"use client";

import { Badge } from "@/components/ui/badge";

export function OperationStatus({ status }: { status: string }) {
  return <Badge variant="outline">{status}</Badge>;
}