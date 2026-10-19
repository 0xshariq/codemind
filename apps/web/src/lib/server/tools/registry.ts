import { APPROVED_TOOL_NAMES, TOOL_CATALOG, type ToolDefinition, type ToolName } from "./catalog";

const definitions = new Map(TOOL_CATALOG.map((tool) => [tool.name, tool]));
if (definitions.size !== APPROVED_TOOL_NAMES.length || APPROVED_TOOL_NAMES.some((name) => !definitions.has(name))) {
  throw new Error("semantic tool catalog and registry are inconsistent");
}

export function getTool(name: string): ToolDefinition {
  const tool = definitions.get(name as ToolName);
  if (!tool) throw new Error(`unsupported semantic tool: ${name}`);
  return tool;
}

export function toolNames(): readonly ToolName[] {
  return APPROVED_TOOL_NAMES;
}
