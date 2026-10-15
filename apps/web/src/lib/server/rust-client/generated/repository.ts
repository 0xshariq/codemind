import type * as grpc from '@grpc/grpc-js';
import type { EnumTypeDefinition, MessageTypeDefinition } from '@grpc/proto-loader';

import type { CanonicalResult as _codemind_v1_CanonicalResult, CanonicalResult__Output as _codemind_v1_CanonicalResult__Output } from './codemind/v1/CanonicalResult';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from './codemind/v1/Correlation';
import type { Diagnostic as _codemind_v1_Diagnostic, Diagnostic__Output as _codemind_v1_Diagnostic__Output } from './codemind/v1/Diagnostic';
import type { ErrorStatus as _codemind_v1_ErrorStatus, ErrorStatus__Output as _codemind_v1_ErrorStatus__Output } from './codemind/v1/ErrorStatus';
import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from './codemind/v1/RepositoryContext';
import type { RepositoryServiceClient as _codemind_v1_RepositoryServiceClient, RepositoryServiceDefinition as _codemind_v1_RepositoryServiceDefinition } from './codemind/v1/RepositoryService';
import type { RepositorySnapshot as _codemind_v1_RepositorySnapshot, RepositorySnapshot__Output as _codemind_v1_RepositorySnapshot__Output } from './codemind/v1/RepositorySnapshot';
import type { ResolveRepositoryRequest as _codemind_v1_ResolveRepositoryRequest, ResolveRepositoryRequest__Output as _codemind_v1_ResolveRepositoryRequest__Output } from './codemind/v1/ResolveRepositoryRequest';
import type { ResolveRepositoryResponse as _codemind_v1_ResolveRepositoryResponse, ResolveRepositoryResponse__Output as _codemind_v1_ResolveRepositoryResponse__Output } from './codemind/v1/ResolveRepositoryResponse';
import type { SourceReference as _codemind_v1_SourceReference, SourceReference__Output as _codemind_v1_SourceReference__Output } from './codemind/v1/SourceReference';

type SubtypeConstructor<Constructor extends new (...args: any) => any, Subtype> = {
  new(...args: ConstructorParameters<Constructor>): Subtype;
};

export interface ProtoGrpcType {
  codemind: {
    v1: {
      CanonicalResult: MessageTypeDefinition<_codemind_v1_CanonicalResult, _codemind_v1_CanonicalResult__Output>
      Completeness: EnumTypeDefinition
      Correlation: MessageTypeDefinition<_codemind_v1_Correlation, _codemind_v1_Correlation__Output>
      Diagnostic: MessageTypeDefinition<_codemind_v1_Diagnostic, _codemind_v1_Diagnostic__Output>
      ErrorStatus: MessageTypeDefinition<_codemind_v1_ErrorStatus, _codemind_v1_ErrorStatus__Output>
      Freshness: EnumTypeDefinition
      Outcome: EnumTypeDefinition
      RepositoryContext: MessageTypeDefinition<_codemind_v1_RepositoryContext, _codemind_v1_RepositoryContext__Output>
      RepositoryService: SubtypeConstructor<typeof grpc.Client, _codemind_v1_RepositoryServiceClient> & { service: _codemind_v1_RepositoryServiceDefinition }
      RepositorySnapshot: MessageTypeDefinition<_codemind_v1_RepositorySnapshot, _codemind_v1_RepositorySnapshot__Output>
      ResolveRepositoryRequest: MessageTypeDefinition<_codemind_v1_ResolveRepositoryRequest, _codemind_v1_ResolveRepositoryRequest__Output>
      ResolveRepositoryResponse: MessageTypeDefinition<_codemind_v1_ResolveRepositoryResponse, _codemind_v1_ResolveRepositoryResponse__Output>
      SourceReference: MessageTypeDefinition<_codemind_v1_SourceReference, _codemind_v1_SourceReference__Output>
    }
  }
}

