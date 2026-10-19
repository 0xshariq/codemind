// Original file: ../../proto/v1/operation.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';

export interface StartInvestigationResponse {
  'operationId'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'selectedEntityId'?: (string);
}

export interface StartInvestigationResponse__Output {
  'operationId'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'selectedEntityId'?: (string);
}
