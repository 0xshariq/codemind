// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/memory.proto

import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from '../../codemind/v1/RepositoryContext';
import type { SourceReference as _codemind_v1_SourceReference, SourceReference__Output as _codemind_v1_SourceReference__Output } from '../../codemind/v1/SourceReference';
import type { Long } from '@grpc/proto-loader';

export interface MemoryRecord {
  'memory_id'?: (string);
  'owner_id'?: (string);
  'kind'?: (string);
  'scope'?: (string);
  'lifecycle'?: (string);
  'freshness'?: (string);
  'payload_json'?: (string);
  'context'?: (_codemind_v1_RepositoryContext | null);
  'evidence'?: (_codemind_v1_SourceReference)[];
  'idempotency_key'?: (string);
  'version'?: (number | string | Long);
  'created_at'?: (string);
  'updated_at'?: (string);
}

export interface MemoryRecord__Output {
  'memory_id'?: (string);
  'owner_id'?: (string);
  'kind'?: (string);
  'scope'?: (string);
  'lifecycle'?: (string);
  'freshness'?: (string);
  'payload_json'?: (string);
  'context'?: (_codemind_v1_RepositoryContext__Output);
  'evidence'?: (_codemind_v1_SourceReference__Output)[];
  'idempotency_key'?: (string);
  'version'?: (string);
  'created_at'?: (string);
  'updated_at'?: (string);
}
