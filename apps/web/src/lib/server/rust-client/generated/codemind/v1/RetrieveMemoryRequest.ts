// Original file: ../../proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface RetrieveMemoryRequest {
  'context'?: (_codemind_v1_RepositoryContext | null);
  'scope'?: (string);
  'kind'?: (string);
  'target'?: (string);
  'limit'?: (number);
  'ownerId'?: (string);
}

export interface RetrieveMemoryRequest__Output {
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'scope'?: (string);
  'kind'?: (string);
  'target'?: (string);
  'limit'?: (number);
  'ownerId'?: (string);
}
