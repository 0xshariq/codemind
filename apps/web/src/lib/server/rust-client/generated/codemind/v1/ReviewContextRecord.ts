// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface ReviewContextRecord {
  'context_id'?: (string);
  'scope'?: (string);
  'payload_json'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'updated_at'?: (string);
}

export interface ReviewContextRecord__Output {
  'context_id'?: (string);
  'scope'?: (string);
  'payload_json'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'updated_at'?: (string);
}
