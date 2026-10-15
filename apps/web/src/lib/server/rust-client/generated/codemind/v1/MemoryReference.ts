// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface MemoryReference {
  'memory_id'?: (string);
  'scope'?: (string);
  'content'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
}

export interface MemoryReference__Output {
  'memory_id'?: (string);
  'scope'?: (string);
  'content'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
}
