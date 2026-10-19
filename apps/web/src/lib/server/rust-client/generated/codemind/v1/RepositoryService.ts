// Original file: ../../proto/v1/repository.proto

import type * as grpc from '@grpc/grpc-js'
import type { MethodDefinition } from '@grpc/proto-loader'
import type { ResolveRepositoryRequest as _codemind_v1_ResolveRepositoryRequest, ResolveRepositoryRequest__Output as _codemind_v1_ResolveRepositoryRequest__Output } from '../../codemind/v1/ResolveRepositoryRequest';
import type { ResolveRepositoryResponse as _codemind_v1_ResolveRepositoryResponse, ResolveRepositoryResponse__Output as _codemind_v1_ResolveRepositoryResponse__Output } from '../../codemind/v1/ResolveRepositoryResponse';

export interface RepositoryServiceClient extends grpc.Client {
  ResolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  ResolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  ResolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  ResolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  resolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  resolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  resolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  resolveRepository(argument: _codemind_v1_ResolveRepositoryRequest, callback: grpc.requestCallback<_codemind_v1_ResolveRepositoryResponse__Output>): grpc.ClientUnaryCall;
  
}

export interface RepositoryServiceHandlers extends grpc.UntypedServiceImplementation {
  ResolveRepository: grpc.handleUnaryCall<_codemind_v1_ResolveRepositoryRequest__Output, _codemind_v1_ResolveRepositoryResponse>;
  
}

export interface RepositoryServiceDefinition extends grpc.ServiceDefinition {
  ResolveRepository: MethodDefinition<_codemind_v1_ResolveRepositoryRequest, _codemind_v1_ResolveRepositoryResponse, _codemind_v1_ResolveRepositoryRequest__Output, _codemind_v1_ResolveRepositoryResponse__Output>
}
