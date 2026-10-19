import type { ReactNode } from "react";
import { EmptyState } from "@/components/common/empty-state";

export type DataTableColumn<Row> = { key: string; header: string; cell: (row: Row) => ReactNode; className?: string };

export function DataTable<Row>({
  columns,
  rows,
  rowKey,
  caption,
  emptyTitle = "No rows",
  emptyDescription = "No data is available for this view.",
}: {
  columns: readonly DataTableColumn<Row>[];
  rows: readonly Row[];
  rowKey: (row: Row) => string;
  caption: string;
  emptyTitle?: string;
  emptyDescription?: string;
}) {
  if (rows.length === 0) return <EmptyState title={emptyTitle} description={emptyDescription} />;
  return <div className="w-full overflow-x-auto rounded-lg border border-border">
    <table className="w-full min-w-[36rem] border-collapse text-left text-sm">
      <caption className="sr-only">{caption}</caption>
      <thead className="bg-surface-alt text-xs text-muted-foreground"><tr>{columns.map((column) => <th key={column.key} scope="col" className={`px-3 py-2.5 font-medium ${column.className ?? ""}`}>{column.header}</th>)}</tr></thead>
      <tbody className="divide-y divide-border">{rows.slice(0, 1_000).map((row) => <tr key={rowKey(row)} className="bg-surface align-top">{columns.map((column) => <td key={column.key} className={`px-3 py-3 ${column.className ?? ""}`}>{column.cell(row)}</td>)}</tr>)}</tbody>
    </table>
  </div>;
}
