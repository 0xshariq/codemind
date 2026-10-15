import type * as grpc from '@grpc/grpc-js';
import type { EnumTypeDefinition, MessageTypeDefinition } from '@grpc/proto-loader';

import type { CanonicalResult as _codemind_v1_CanonicalResult, CanonicalResult__Output as _codemind_v1_CanonicalResult__Output } from './codemind/v1/CanonicalResult';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from './codemind/v1/Correlation';
import type { Diagnostic as _codemind_v1_Diagnostic, Diagnostic__Output as _codemind_v1_Diagnostic__Output } from './codemind/v1/Diagnostic';
import type { ErrorStatus as _codemind_v1_ErrorStatus, ErrorStatus__Output as _codemind_v1_ErrorStatus__Output } from './codemind/v1/ErrorStatus';
import type { IntelligenceServiceClient as _codemind_v1_IntelligenceServiceClient, IntelligenceServiceDefinition as _codemind_v1_IntelligenceServiceDefinition } from './codemind/v1/IntelligenceService';
import type { QueryResponse as _codemind_v1_QueryResponse, QueryResponse__Output as _codemind_v1_QueryResponse__Output } from './codemind/v1/QueryResponse';
import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from './codemind/v1/RepositoryContext';
import type { SourceReference as _codemind_v1_SourceReference, SourceReference__Output as _codemind_v1_SourceReference__Output } from './codemind/v1/SourceReference';
import type { SymbolRelationshipQuery as _codemind_v1_SymbolRelationshipQuery, SymbolRelationshipQuery__Output as _codemind_v1_SymbolRelationshipQuery__Output } from './codemind/v1/SymbolRelationshipQuery';

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
      IntelligenceService: SubtypeConstructor<typeof grpc.Client, _codemind_v1_IntelligenceServiceClient> & { service: _codemind_v1_IntelligenceServiceDefinition }
      Outcome: EnumTypeDefinition
      QueryResponse: MessageTypeDefinition<_codemind_v1_QueryResponse, _codemind_v1_QueryResponse__Output>
      RepositoryContext: MessageTypeDefinition<_codemind_v1_RepositoryContext, _codemind_v1_RepositoryContext__Output>
      SourceReference: MessageTypeDefinition<_codemind_v1_SourceReference, _codemind_v1_SourceReference__Output>
      SymbolRelationshipQuery: MessageTypeDefinition<_codemind_v1_SymbolRelationshipQuery, _codemind_v1_SymbolRelationshipQuery__Output>
    }
  }
}

