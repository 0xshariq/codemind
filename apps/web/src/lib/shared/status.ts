export const OPERATION_LIFECYCLES = ["pending", "running", "completed", "failed"] as const;
export type OperationLifecycle = (typeof OPERATION_LIFECYCLES)[number];

export const OUTCOMES = ["success", "empty", "partial", "unsupported", "invalid", "unavailable", "failed"] as const;
export type Outcome = (typeof OUTCOMES)[number];

export const COMPLETENESS_STATES = ["complete", "truncated", "incomplete"] as const;
export type Completeness = (typeof COMPLETENESS_STATES)[number];

export const FRESHNESS_STATES = ["current", "stale", "unknown"] as const;
export type Freshness = (typeof FRESHNESS_STATES)[number];

export const MEMORY_LIFECYCLES = ["candidate", "validated", "rejected", "superseded"] as const;
export type MemoryLifecycle = (typeof MEMORY_LIFECYCLES)[number];

export const HANDOFF_LIFECYCLES = ["pending", "claimed", "consumed", "expired"] as const;
export type HandoffLifecycle = (typeof HANDOFF_LIFECYCLES)[number];

export const REPOSITORY_READINESS = ["unresolved", "resolving", "indexing", "ready", "partial", "unavailable"] as const;
export type RepositoryReadiness = (typeof REPOSITORY_READINESS)[number];

export const ERROR_CATEGORIES = ["invalid", "unauthorized", "not_found", "conflict", "unavailable", "timeout", "internal"] as const;
export type ErrorCategory = (typeof ERROR_CATEGORIES)[number];

export type DataStatus = {
  outcome: Outcome;
  completeness: Completeness;
  freshness: Freshness;
};
