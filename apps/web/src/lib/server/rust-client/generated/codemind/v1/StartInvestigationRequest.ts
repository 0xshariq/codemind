// Original file: ../../proto/v1/operation.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from '../../codemind/v1/Correlation';

export interface StartInvestigationRequest {
  'context'?: (_codemind_v1_RepositoryContext | null);
  'sourcePath'?: (string);
  'symbolName'?: (string);
  'correlation'?: (_codemind_v1_Correlation | null);
}

export interface StartInvestigationRequest__Output {
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'sourcePath'?: (string);
  'symbolName'?: (string);
  'correlation'?: (_codemind_v1_Correlation__Output);
}
