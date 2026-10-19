import type { Completeness, ErrorCategory, Freshness, HandoffLifecycle, MemoryLifecycle, OperationLifecycle, Outcome, RepositoryReadiness } from "./status";

export type RepositoryContext = {
  repositoryId: string;
  workspaceId: string;
  sourceSnapshotId: string;
  graphVersion: string;
};

export type RepositoryDto = RepositoryContext & {
  name: string;
  source: "github" | "local";
  readiness: RepositoryReadiness;
  defaultBranch?: string;
  updatedAt: string;
};

export type EvidenceReference = {
  path: string;
  line?: number;
  startLine?: number;
  endLine?: number;
  snapshotId: string;
  excerpt?: string;
};

export type RelationshipResult = {
  id: string;
  kind: string;
  from: string;
  to: string;
  context: RepositoryContext;
  depth?: number;
  evidence?: EvidenceReference[];
};

export type GraphPath = {
  pathId: string;
  entityIds: string[];
  relationships: RelationshipResult[];
  context: RepositoryContext;
};

export type TestImpact = {
  testId: string;
  name: string;
  path: string;
  impact: "direct" | "indirect" | "unknown";
  evidence: EvidenceReference[];
};

export type AnalysisDto = {
  analysisId: string;
  context: RepositoryContext;
  outcome: Outcome;
  completeness: Completeness;
  freshness: Freshness;
  affectedFiles: string[];
  paths: GraphPath[];
  testImpact: TestImpact[];
  evidence: EvidenceReference[];
  diagnostics: string[];
  limitations: string[];
  risk?: { level: "low" | "medium" | "high" | "unknown"; confidence?: number; factors: string[] };
};

export type MemoryDto = {
  memoryId: string;
  ownerId: string;
  kind: "semantic" | "procedure";
  scope: "user" | "repository" | "team";
  lifecycle: MemoryLifecycle;
  freshness: Freshness;
  context: RepositoryContext;
  evidence: EvidenceReference[];
  payload: Record<string, unknown>;
  updatedAt: string;
};

export type HandoffDto = {
  handoffId: string;
  operationId: string;
  sourceStage: string;
  targetStage: string;
  lifecycle: HandoffLifecycle;
  context: RepositoryContext;
  findings: string[];
  evidence: EvidenceReference[];
  openQuestions: string[];
};

export type OperationDto = {
  operationId: string;
  context: RepositoryContext;
  lifecycle: OperationLifecycle;
  outcome: Outcome;
  completeness: Completeness;
  sequence: number;
  stage: string;
  resultReference?: string;
  limitations: string[];
  errorCategory?: ErrorCategory;
  updatedAt: string;
};
