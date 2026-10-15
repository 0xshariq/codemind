// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/operation.proto

import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from '../../codemind/v1/Correlation';
import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { Long } from '@grpc/proto-loader';

export interface ProgressEvent {
  'event_id'?: (string);
  'sequence'?: (number | string | Long);
  'engine'?: (string);
  'stage'?: (string);
  'timestamp'?: (string);
  'correlation'?: (_codemind_v1_Correlation | null);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'message'?: (string);
}

export interface ProgressEvent__Output {
  'event_id'?: (string);
  'sequence'?: (string);
  'engine'?: (string);
  'stage'?: (string);
  'timestamp'?: (string);
  'correlation'?: (_codemind_v1_Correlation__Output);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'message'?: (string);
}
