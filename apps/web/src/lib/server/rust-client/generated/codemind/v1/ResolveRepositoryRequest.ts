// Original file: ../../proto/v1/repository.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from '../../codemind/v1/Correlation';

export interface ResolveRepositoryRequest {
  'sourcePath'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'correlation'?: (_codemind_v1_Correlation | null);
}

export interface ResolveRepositoryRequest__Output {
  'sourcePath'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'correlation'?: (_codemind_v1_Correlation__Output);
}
