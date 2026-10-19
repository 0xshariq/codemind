// Original file: ../../proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface RetrieveHandoffRequest {
  'handoffId'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
}

export interface RetrieveHandoffRequest__Output {
  'handoffId'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
}
