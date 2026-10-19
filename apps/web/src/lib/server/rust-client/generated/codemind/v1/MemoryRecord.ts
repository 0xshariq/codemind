// Original file: ../../proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { SourceReference as _codemind_v1_SourceReference, SourceReference__Output as _codemind_v1_SourceReference__Output } from '../../codemind/v1/SourceReference';
import type { Long } from '@grpc/proto-loader';

export interface MemoryRecord {
  'memoryId'?: (string);
  'ownerId'?: (string);
  'kind'?: (string);
  'scope'?: (string);
  'lifecycle'?: (string);
  'freshness'?: (string);
  'payloadJson'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'evidence'?: (_codemind_v1_SourceReference)[];
  'idempotencyKey'?: (string);
  'version'?: (number | string | Long);
  'createdAt'?: (string);
  'updatedAt'?: (string);
}

export interface MemoryRecord__Output {
  'memoryId'?: (string);
  'ownerId'?: (string);
  'kind'?: (string);
  'scope'?: (string);
  'lifecycle'?: (string);
  'freshness'?: (string);
  'payloadJson'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'evidence'?: (_codemind_v1_SourceReference__Output)[];
  'idempotencyKey'?: (string);
  'version'?: (string);
  'createdAt'?: (string);
  'updatedAt'?: (string);
}
