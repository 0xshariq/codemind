import { z } from "zod";

export const contextSchema = z.object({
  workspaceId: z.string().min(1),
  repositoryId: z.string().min(1),
  sourceSnapshotId: z.string().min(1),
  graphVersion: z.string().min(1),
});

export const toolInputSchema = z.object({
  context: contextSchema,
  symbolId: z.string().min(1).optional(),
  query: z.string().min(1).max(500).optional(),
  limit: z.number().int().min(1).max(100).default(20),
});

export type ToolInput = z.infer<typeof toolInputSchema>;