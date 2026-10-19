// Original file: ../../proto/v1/memory.proto

import type { Handoff as _codemind_v1_Handoff, Handoff__Output as _codemind_v1_Handoff__Output } from '../../codemind/v1/Handoff';

export interface PersistHandoffRequest {
  'handoff'?: (_codemind_v1_Handoff | null);
  'idempotencyKey'?: (string);
}

export interface PersistHandoffRequest__Output {
  'handoff'?: (_codemind_v1_Handoff__Output);
  'idempotencyKey'?: (string);
}
