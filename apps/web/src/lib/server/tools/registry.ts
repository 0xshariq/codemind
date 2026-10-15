import { APPROVED_TOOL_NAMES, TOOL_CATALOG, type ToolDefinition, type ToolName } from "./catalog";

export function getTool(name: string): ToolDefinition {
  const tool = TOOL_CATALOG.find((candidate) => candidate.name === name);
  if (!tool) throw new Error(`unsupported semantic tool: ${name}`);
  return tool;
}

export function toolNames(): readonly ToolName[] {
  return APPROVED_TOOL_NAMES;
}