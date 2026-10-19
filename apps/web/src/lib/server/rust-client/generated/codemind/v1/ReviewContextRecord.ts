// Original file: ../../proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface ReviewContextRecord {
  'contextId'?: (string);
  'scope'?: (string);
  'payloadJson'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'updatedAt'?: (string);
}

export interface ReviewContextRecord__Output {
  'contextId'?: (string);
  'scope'?: (string);
  'payloadJson'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'updatedAt'?: (string);
}
