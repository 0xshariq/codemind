// Original file: ../../proto/v1/intelligence.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from '../../codemind/v1/Correlation';

export interface ChangeAnalysisQuery {
  'symbolId'?: (string);
  'maxDepth'?: (number);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'correlation'?: (_codemind_v1_Correlation | null);
}

export interface ChangeAnalysisQuery__Output {
  'symbolId'?: (string);
  'maxDepth'?: (number);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'correlation'?: (_codemind_v1_Correlation__Output);
}
