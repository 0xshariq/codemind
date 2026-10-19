import { getRepository } from "../../repositories/service";
import type { ToolInput } from "../schemas";

export async function getRepositoryContext(input: ToolInput) {
  const repository = await getRepository(input.context.workspaceId, input.context.repositoryId);
  if (!repository) return {
    outcome: "unavailable", completeness: "incomplete", freshness: "unknown", context: input.context,
    data: null, evidence: [], diagnostics: [], limitations: ["Rust could not resolve this repository context."],
  };
  if (repository.sourceSnapshotId !== input.context.sourceSnapshotId || repository.graphVersion !== input.context.graphVersion) {
    return {
      outcome: "invalid", completeness: "incomplete", freshness: "stale", context: input.context,
      data: { repositoryId: repository.repositoryId, readiness: repository.readiness }, evidence: [], diagnostics: [],
      limitations: ["The requested context does not match the repository's current Rust snapshot."],
    };
  }
  return {
    outcome: repository.readiness === "ready" ? "success" : "partial",
    completeness: repository.readiness === "ready" ? "complete" : "incomplete",
    freshness: "current", context: input.context,
    data: {
      repositoryId: repository.repositoryId, name: repository.name, source: repository.source,
      readiness: repository.readiness, fileCount: repository.fileCount,
    }, evidence: [], diagnostics: [], limitations: repository.limitations,
  };
}
