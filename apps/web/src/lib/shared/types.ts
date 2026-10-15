export type RepositoryContext = {
  repositoryId: string;
  workspaceId: string;
  sourceSnapshotId: string;
  graphVersion: string;
};

export type RelationshipResult = {
  id: string;
  kind: string;
  from: string;
  to: string;
  context: RepositoryContext;
};