// Original file: ../../proto/v1/memory.proto

import type { MemoryRecord as _codemind_v1_MemoryRecord, MemoryRecord__Output as _codemind_v1_MemoryRecord__Output } from '../../codemind/v1/MemoryRecord';
import type { Handoff as _codemind_v1_Handoff, Handoff__Output as _codemind_v1_Handoff__Output } from '../../codemind/v1/Handoff';
import type { ErrorStatus as _codemind_v1_ErrorStatus, ErrorStatus__Output as _codemind_v1_ErrorStatus__Output } from '../../codemind/v1/ErrorStatus';
import type { ReviewContextRecord as _codemind_v1_ReviewContextRecord, ReviewContextRecord__Output as _codemind_v1_ReviewContextRecord__Output } from '../../codemind/v1/ReviewContextRecord';

export interface MemoryResponse {
  'memory'?: (_codemind_v1_MemoryRecord | null);
  'handoff'?: (_codemind_v1_Handoff | null);
  'error'?: (_codemind_v1_ErrorStatus | null);
  'review'?: (_codemind_v1_ReviewContextRecord | null);
  'result'?: "memory"|"handoff"|"error"|"review";
}

export interface MemoryResponse__Output {
  'memory'?: (_codemind_v1_MemoryRecord__Output);
  'handoff'?: (_codemind_v1_Handoff__Output);
  'error'?: (_codemind_v1_ErrorStatus__Output);
  'review'?: (_codemind_v1_ReviewContextRecord__Output);
  'result'?: "memory"|"handoff"|"error"|"review";
}
