import { DataTable, type DataTableColumn } from "@/components/ui/data-table";

export type AffectedFile = { id: string; path: string; symbol?: string; relation: string; completeness?: string; evidence?: string };

export function AffectedFilesPanel({ files }: { files: readonly AffectedFile[] }) {
  const columns: DataTableColumn<AffectedFile>[] = [
    { key: "path", header: "File", cell: (file) => <code className="break-all font-mono text-xs">{file.path}</code> },
    { key: "symbol", header: "Symbol", cell: (file) => file.symbol ? <code className="break-all font-mono text-xs">{file.symbol}</code> : <span className="text-xs text-muted-foreground">File-level</span> },
    { key: "relation", header: "Relationship", cell: (file) => <span className="text-xs">{file.relation.replaceAll("_", " ")}</span> },
    { key: "status", header: "Status", cell: (file) => <span className="text-xs text-muted-foreground">{file.completeness?.replaceAll("_", " ") ?? "Unknown"}</span> },
    { key: "evidence", header: "Evidence", cell: (file) => file.evidence ? <code className="break-all font-mono text-xs text-muted-foreground">{file.evidence}</code> : <span className="text-xs text-muted-foreground">Not supplied</span> },
  ];
  return <section className="grid gap-3" aria-label="Affected files and symbols"><h2 className="text-base font-semibold">Affected files and symbols</h2><DataTable columns={columns} rows={files} rowKey={(file) => file.id} caption="Canonical affected files and symbols" emptyTitle="No affected files" emptyDescription="No affected file or symbol records were returned." /></section>;
}
