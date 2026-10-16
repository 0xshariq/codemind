import { z } from "zod";

export const handoffSchema = z.object({
  handoffId: z.string().min(1),
  schemaVersion: z.string().min(1),
  operationId: z.string().min(1),
  reviewEpisodeId: z.string().min(1),
  sourceStage: z.string().min(1),
  targetStage: z.string().min(1),
  task: z.string().min(1).max(4000),
  findings: z.array(z.string().min(1)).max(32),
  evidenceReferences: z.array(z.object({
    path: z.string().min(1),
    startLine: z.number().int().nonnegative(),
    endLine: z.number().int().nonnegative(),
    snapshotId: z.string().min(1),
  })),
  context: z.object({
    repositoryId: z.string().min(1),
    workspaceId: z.string().min(1),
    sourceSnapshotId: z.string().min(1),
    graphVersion: z.string().min(1),
  }),
  openQuestions: z.array(z.string()).max(32),
  status: z.enum(["pending", "claimed", "consumed", "expired"]),
  idempotencyKey: z.string().min(1),
  version: z.number().int().nonnegative(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type Handoff = z.infer<typeof handoffSchema>;