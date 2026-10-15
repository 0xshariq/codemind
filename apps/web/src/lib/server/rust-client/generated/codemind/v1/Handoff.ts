// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface Handoff {
  'handoff_id'?: (string);
  'status'?: (string);
  'payload_json'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
}

export interface Handoff__Output {
  'handoff_id'?: (string);
  'status'?: (string);
  'payload_json'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
}
