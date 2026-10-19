import { ArrowRight, Network } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { Badge } from "@/components/ui/badge";

export type GraphEdge = { id: string; from: string; to: string; relationship: string; depth?: number };

export function DependencyGraph({
  edges,
  truncated = false,
  selectedPathId,
  onSelectPath,
}: {
  edges: readonly GraphEdge[];
  truncated?: boolean;
  selectedPathId?: string;
  onSelectPath?: (id: string) => void;
}) {
  if (!edges.length) return <EmptyState title="No graph paths returned" description="The canonical result contains no supported dependency edges to display." icon={<Network className="size-6" />} />;
  const visibleEdges = edges.slice(0, 50);
  const positions = new Map<string, { x: number; y: number; label: string }>();
  const layerRows = new Map<number, number>();
  const addNode = (id: string, layer: number) => {
    const existing = positions.get(id);
    if (existing) return existing;
    const row = layerRows.get(layer) ?? 0;
    layerRows.set(layer, row + 1);
    const node = { x: 120 + layer * 240, y: 42 + row * 84, label: id };
    positions.set(id, node);
    return node;
  };
  const laidOut = visibleEdges.map((edge) => {
    const layer = Math.max(0, Math.min(8, edge.depth ?? 0));
    return { edge, from: addNode(edge.from, layer), to: addNode(edge.to, Math.min(layer + 1, 9)) };
  });
  const width = Math.max(240, ...[...positions.values()].map((node) => node.x + 120));
  const height = Math.max(120, ...[...layerRows.values()].map((count) => 42 + count * 84));
  return <section className="grid gap-3" aria-label="Dependency graph paths">
    <div className="flex flex-wrap items-center gap-2"><h2 className="text-base font-semibold">Dependency paths</h2><Badge variant="outline">{Math.min(edges.length, 500)} shown</Badge>{truncated && <Badge variant="secondary">Truncated</Badge>}</div>
    <p className="text-xs text-muted-foreground">Text path view preserves edge direction and remains available to assistive technology.</p>
    <div className="overflow-x-auto rounded-lg border border-border bg-background p-2" role="img" aria-label={`Directed dependency graph with ${visibleEdges.length} edges`}>
      <svg aria-hidden="true" viewBox={`0 0 ${width} ${height}`} className="h-auto min-h-32 w-full" style={{ minWidth: `${Math.min(width, 900)}px` }}>
        <defs><marker id="dependency-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor" className="text-primary" /></marker></defs>
        {laidOut.map(({ edge, from, to }) => <g key={`edge-${edge.id}`}><line x1={from.x + 90} y1={from.y} x2={to.x - 90} y2={to.y} stroke="currentColor" strokeWidth="1.5" markerEnd="url(#dependency-arrow)" className="text-primary/70" /><text x={(from.x + to.x) / 2} y={(from.y + to.y) / 2 - 6} textAnchor="middle" className="fill-muted-foreground" fontSize="10">{edge.relationship.replaceAll("_", " ")}</text></g>)}
        {[...positions.entries()].map(([id, node]) => <g key={`node-${id}`}><rect x={node.x - 90} y={node.y - 22} width="180" height="44" rx="8" fill="currentColor" className="text-surface" stroke="currentColor" /><rect x={node.x - 90} y={node.y - 22} width="180" height="44" rx="8" fill="none" stroke="currentColor" className="text-border" /><text x={node.x} y={node.y + 4} textAnchor="middle" className="fill-foreground" fontSize="11">{node.label.length > 26 ? `${node.label.slice(0, 23)}…` : node.label}<title>{id}</title></text></g>)}
      </svg>
    </div>
    <ol className="grid gap-2">{edges.slice(0, 500).map((edge) => <li key={edge.id}><button type="button" onClick={() => onSelectPath?.(edge.id)} disabled={!onSelectPath} aria-pressed={selectedPathId === edge.id} className={`grid w-full gap-2 rounded-lg border p-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default ${selectedPathId === edge.id ? "border-primary bg-primary/5" : "border-border bg-surface"}`}><div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"><Badge variant="outline">{edge.relationship.replaceAll("_", " ")}</Badge>{edge.depth !== undefined && <span>depth {edge.depth}</span>}</div><div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2"><code className="break-all font-mono text-xs">{edge.from}</code><ArrowRight aria-hidden="true" className="size-4 text-primary" /><code className="break-all font-mono text-xs">{edge.to}</code></div></button></li>)}</ol>
  </section>;
}
