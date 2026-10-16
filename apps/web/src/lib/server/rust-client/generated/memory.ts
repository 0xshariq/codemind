import type * as grpc from '@grpc/grpc-js';
import type { EnumTypeDefinition, MessageTypeDefinition } from '@grpc/proto-loader';

import type { CanonicalResult as _codemind_v1_CanonicalResult, CanonicalResult__Output as _codemind_v1_CanonicalResult__Output } from './codemind/v1/CanonicalResult';
import type { Correlation as _codemind_v1_Correlation, Correlation__Output as _codemind_v1_Correlation__Output } from './codemind/v1/Correlation';
import type { Diagnostic as _codemind_v1_Diagnostic, Diagnostic__Output as _codemind_v1_Diagnostic__Output } from './codemind/v1/Diagnostic';
import type { ErrorStatus as _codemind_v1_ErrorStatus, ErrorStatus__Output as _codemind_v1_ErrorStatus__Output } from './codemind/v1/ErrorStatus';
import type { Handoff as _codemind_v1_Handoff, Handoff__Output as _codemind_v1_Handoff__Output } from './codemind/v1/Handoff';
import type { MemoryRecord as _codemind_v1_MemoryRecord, MemoryRecord__Output as _codemind_v1_MemoryRecord__Output } from './codemind/v1/MemoryRecord';
import type { MemoryReference as _codemind_v1_MemoryReference, MemoryReference__Output as _codemind_v1_MemoryReference__Output } from './codemind/v1/MemoryReference';
import type { MemoryResponse as _codemind_v1_MemoryResponse, MemoryResponse__Output as _codemind_v1_MemoryResponse__Output } from './codemind/v1/MemoryResponse';
import type { MemoryServiceClient as _codemind_v1_MemoryServiceClient, MemoryServiceDefinition as _codemind_v1_MemoryServiceDefinition } from './codemind/v1/MemoryService';
import type { PersistHandoffRequest as _codemind_v1_PersistHandoffRequest, PersistHandoffRequest__Output as _codemind_v1_PersistHandoffRequest__Output } from './codemind/v1/PersistHandoffRequest';
import type { PersistMemoryRequest as _codemind_v1_PersistMemoryRequest, PersistMemoryRequest__Output as _codemind_v1_PersistMemoryRequest__Output } from './codemind/v1/PersistMemoryRequest';
import type { PersistReviewContextRequest as _codemind_v1_PersistReviewContextRequest, PersistReviewContextRequest__Output as _codemind_v1_PersistReviewContextRequest__Output } from './codemind/v1/PersistReviewContextRequest';
import type { RepositoryContext as _codemind_v1_RepositoryContext, RepositoryContext__Output as _codemind_v1_RepositoryContext__Output } from './codemind/v1/RepositoryContext';
import type { RetrieveHandoffRequest as _codemind_v1_RetrieveHandoffRequest, RetrieveHandoffRequest__Output as _codemind_v1_RetrieveHandoffRequest__Output } from './codemind/v1/RetrieveHandoffRequest';
import type { RetrieveMemoryRequest as _codemind_v1_RetrieveMemoryRequest, RetrieveMemoryRequest__Output as _codemind_v1_RetrieveMemoryRequest__Output } from './codemind/v1/RetrieveMemoryRequest';
import type { RetrieveMemoryResponse as _codemind_v1_RetrieveMemoryResponse, RetrieveMemoryResponse__Output as _codemind_v1_RetrieveMemoryResponse__Output } from './codemind/v1/RetrieveMemoryResponse';
import type { RetrieveReviewContextRequest as _codemind_v1_RetrieveReviewContextRequest, RetrieveReviewContextRequest__Output as _codemind_v1_RetrieveReviewContextRequest__Output } from './codemind/v1/RetrieveReviewContextRequest';
import type { ReviewContextRecord as _codemind_v1_ReviewContextRecord, ReviewContextRecord__Output as _codemind_v1_ReviewContextRecord__Output } from './codemind/v1/ReviewContextRecord';
import type { SourceReference as _codemind_v1_SourceReference, SourceReference__Output as _codemind_v1_SourceReference__Output } from './codemind/v1/SourceReference';
import type { TransitionHandoffRequest as _codemind_v1_TransitionHandoffRequest, TransitionHandoffRequest__Output as _codemind_v1_TransitionHandoffRequest__Output } from './codemind/v1/TransitionHandoffRequest';

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
      Handoff: MessageTypeDefinition<_codemind_v1_Handoff, _codemind_v1_Handoff__Output>
      MemoryRecord: MessageTypeDefinition<_codemind_v1_MemoryRecord, _codemind_v1_MemoryRecord__Output>
      MemoryReference: MessageTypeDefinition<_codemind_v1_MemoryReference, _codemind_v1_MemoryReference__Output>
      MemoryResponse: MessageTypeDefinition<_codemind_v1_MemoryResponse, _codemind_v1_MemoryResponse__Output>
      MemoryService: SubtypeConstructor<typeof grpc.Client, _codemind_v1_MemoryServiceClient> & { service: _codemind_v1_MemoryServiceDefinition }
      Outcome: EnumTypeDefinition
      PersistHandoffRequest: MessageTypeDefinition<_codemind_v1_PersistHandoffRequest, _codemind_v1_PersistHandoffRequest__Output>
      PersistMemoryRequest: MessageTypeDefinition<_codemind_v1_PersistMemoryRequest, _codemind_v1_PersistMemoryRequest__Output>
      PersistReviewContextRequest: MessageTypeDefinition<_codemind_v1_PersistReviewContextRequest, _codemind_v1_PersistReviewContextRequest__Output>
      RepositoryContext: MessageTypeDefinition<_codemind_v1_RepositoryContext, _codemind_v1_RepositoryContext__Output>
      RetrieveHandoffRequest: MessageTypeDefinition<_codemind_v1_RetrieveHandoffRequest, _codemind_v1_RetrieveHandoffRequest__Output>
      RetrieveMemoryRequest: MessageTypeDefinition<_codemind_v1_RetrieveMemoryRequest, _codemind_v1_RetrieveMemoryRequest__Output>
      RetrieveMemoryResponse: MessageTypeDefinition<_codemind_v1_RetrieveMemoryResponse, _codemind_v1_RetrieveMemoryResponse__Output>
      RetrieveReviewContextRequest: MessageTypeDefinition<_codemind_v1_RetrieveReviewContextRequest, _codemind_v1_RetrieveReviewContextRequest__Output>
      ReviewContextRecord: MessageTypeDefinition<_codemind_v1_ReviewContextRecord, _codemind_v1_ReviewContextRecord__Output>
      SourceReference: MessageTypeDefinition<_codemind_v1_SourceReference, _codemind_v1_SourceReference__Output>
      TransitionHandoffRequest: MessageTypeDefinition<_codemind_v1_TransitionHandoffRequest, _codemind_v1_TransitionHandoffRequest__Output>
    }
  }
}

