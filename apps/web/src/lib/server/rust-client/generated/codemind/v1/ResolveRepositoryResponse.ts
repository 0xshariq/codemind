// Original file: ../../proto/v1/repository.proto

import type { RepositorySnapshot as _codemind_v1_RepositorySnapshot, RepositorySnapshot__Output as _codemind_v1_RepositorySnapshot__Output } from '../../codemind/v1/RepositorySnapshot';
import type { ErrorStatus as _codemind_v1_ErrorStatus, ErrorStatus__Output as _codemind_v1_ErrorStatus__Output } from '../../codemind/v1/ErrorStatus';

export interface ResolveRepositoryResponse {
  'snapshot'?: (_codemind_v1_RepositorySnapshot | null);
  'error'?: (_codemind_v1_ErrorStatus | null);
  'result'?: "snapshot"|"error";
}

export interface ResolveRepositoryResponse__Output {
  'snapshot'?: (_codemind_v1_RepositorySnapshot__Output);
  'error'?: (_codemind_v1_ErrorStatus__Output);
  'result'?: "snapshot"|"error";
}
