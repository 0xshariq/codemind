import { z } from "zod";

const identity = z.string().trim().min(1).max(1_024).refine((value) => !/[\u0000-\u001f\u007f]/.test(value));
const handoffShape = z.object({
  handoffId: identity,
  schemaVersion: z.string().trim().min(1).max(64),
  operationId: identity,
  reviewEpisodeId: identity,
  sourceStage: z.string().trim().min(1).max(128),
  targetStage: z.string().trim().min(1).max(128),
  task: z.string().min(1).max(4000),
  findings: z.array(z.string().min(1)).max(32),
  evidenceReferences: z.array(z.object({
    path: z.string().trim().min(1).max(2_048).refine((value) => !value.startsWith("/") && !value.split(/[\\/]/).includes("..")),
    startLine: z.number().int().positive(),
    endLine: z.number().int().positive(),
    snapshotId: identity,
  }).strict()).max(128),
  context: z.object({
    repositoryId: z.string().min(1),
    workspaceId: z.string().min(1),
    sourceSnapshotId: z.string().min(1),
    graphVersion: z.string().min(1),
  }),
  openQuestions: z.array(z.string().trim().min(1).max(2_000)).max(32),
  status: z.enum(["pending", "claimed", "consumed", "expired"]),
  idempotencyKey: z.string().trim().min(1).max(256),
  version: z.number().int().nonnegative(),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
}).strict();

export const handoffSchema = handoffShape.superRefine((handoff, context) => {
  if (handoff.evidenceReferences.some((reference) => reference.endLine < reference.startLine || reference.snapshotId !== handoff.context.sourceSnapshotId)) {
    context.addIssue({ code: "custom", path: ["evidenceReferences"], message: "handoff evidence must match its repository snapshot" });
  }
});

export function canTransitionHandoff(from: Handoff["status"], to: Handoff["status"]) {
  return from === to || (from === "pending" && (to === "claimed" || to === "expired"))
    || (from === "claimed" && (to === "consumed" || to === "expired"));
}

export type Handoff = z.infer<typeof handoffSchema>;
