// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/intelligence.proto

import type * as grpc from '@grpc/grpc-js'
import type { MethodDefinition } from '@grpc/proto-loader'
import type { QueryResponse as _codemind_v1_QueryResponse, QueryResponse__Output as _codemind_v1_QueryResponse__Output } from '../../codemind/v1/QueryResponse';
import type { SymbolRelationshipQuery as _codemind_v1_SymbolRelationshipQuery, SymbolRelationshipQuery__Output as _codemind_v1_SymbolRelationshipQuery__Output } from '../../codemind/v1/SymbolRelationshipQuery';

export interface IntelligenceServiceClient extends grpc.Client {
  QuerySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  QuerySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  QuerySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  QuerySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  querySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  querySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  querySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  querySymbolRelationships(argument: _codemind_v1_SymbolRelationshipQuery, callback: grpc.requestCallback<_codemind_v1_QueryResponse__Output>): grpc.ClientUnaryCall;
  
}

export interface IntelligenceServiceHandlers extends grpc.UntypedServiceImplementation {
  QuerySymbolRelationships: grpc.handleUnaryCall<_codemind_v1_SymbolRelationshipQuery__Output, _codemind_v1_QueryResponse>;
  
}

export interface IntelligenceServiceDefinition extends grpc.ServiceDefinition {
  QuerySymbolRelationships: MethodDefinition<_codemind_v1_SymbolRelationshipQuery, _codemind_v1_QueryResponse, _codemind_v1_SymbolRelationshipQuery__Output, _codemind_v1_QueryResponse__Output>
}
