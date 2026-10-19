// Original file: ../../proto/v1/common.proto

import type { Outcome as _codemind_v1_Outcome, Outcome__Output as _codemind_v1_Outcome__Output } from '../../codemind/v1/Outcome';
import type { Completeness as _codemind_v1_Completeness, Completeness__Output as _codemind_v1_Completeness__Output } from '../../codemind/v1/Completeness';
import type { Freshness as _codemind_v1_Freshness, Freshness__Output as _codemind_v1_Freshness__Output } from '../../codemind/v1/Freshness';
import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from '../../codemind/v1/Correlation';
import type { SourceReference as _codemind_v1_SourceReference, SourceReference__Output as _codemind_v1_SourceReference__Output } from '../../codemind/v1/SourceReference';
import type { Diagnostic as _codemind_v1_Diagnostic, Diagnostic__Output as _codemind_v1_Diagnostic__Output } from '../../codemind/v1/Diagnostic';

export interface CanonicalResult {
  'outcome'?: (_codemind_v1_Outcome);
  'completeness'?: (_codemind_v1_Completeness);
  'freshness'?: (_codemind_v1_Freshness);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'correlation'?: (_codemind_v1_Correlation | null);
  'dataJson'?: (string);
  'evidence'?: (_codemind_v1_SourceReference)[];
  'diagnostics'?: (_codemind_v1_Diagnostic)[];
  'limitations'?: (string)[];
}

export interface CanonicalResult__Output {
  'outcome'?: (_codemind_v1_Outcome__Output);
  'completeness'?: (_codemind_v1_Completeness__Output);
  'freshness'?: (_codemind_v1_Freshness__Output);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'correlation'?: (_codemind_v1_Correlation__Output);
  'dataJson'?: (string);
  'evidence'?: (_codemind_v1_SourceReference__Output)[];
  'diagnostics'?: (_codemind_v1_Diagnostic__Output)[];
  'limitations'?: (string)[];
}
