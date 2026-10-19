// Original file: ../../proto/v1/intelligence.proto

import type { CanonicalResult as _codemind_v1_CanonicalResult, CanonicalResult__Output as _codemind_v1_CanonicalResult__Output } from '../../codemind/v1/CanonicalResult';
import type { ErrorStatus as _codemind_v1_ErrorStatus, ErrorStatus__Output as _codemind_v1_ErrorStatus__Output } from '../../codemind/v1/ErrorStatus';

export interface QueryResponse {
  'data'?: (_codemind_v1_CanonicalResult | null);
  'error'?: (_codemind_v1_ErrorStatus | null);
  'result'?: "data"|"error";
}

export interface QueryResponse__Output {
  'data'?: (_codemind_v1_CanonicalResult__Output);
  'error'?: (_codemind_v1_ErrorStatus__Output);
  'result'?: "data"|"error";
}
