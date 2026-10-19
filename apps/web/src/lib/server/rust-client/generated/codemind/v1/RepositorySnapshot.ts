// Original file: ../../proto/v1/repository.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface RepositorySnapshot {
  'context'?: (_codemind_v1_RepositoryContext | null);
  'repositoryName'?: (string);
  'manifestJson'?: (string);
}

export interface RepositorySnapshot__Output {
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'repositoryName'?: (string);
  'manifestJson'?: (string);
}
