// Original file: ../../proto/v1/operation.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { Long } from '@grpc/proto-loader';

export interface StreamProgressRequest {
  'context'?: (_codemind_v1_RepositoryContext | null);
  'operationId'?: (string);
  'afterSequence'?: (number | string | Long);
  'limit'?: (number);
}

export interface StreamProgressRequest__Output {
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'operationId'?: (string);
  'afterSequence'?: (string);
  'limit'?: (number);
}
