// Original file: ../../proto/v1/memory.proto

import type * as grpc from '@grpc/grpc-js'
import type { MethodDefinition } from '@grpc/proto-loader'
import type { MemoryResponse as _codemind_v1_MemoryResponse, MemoryResponse__Output as _codemind_v1_MemoryResponse__Output } from '../../codemind/v1/MemoryResponse';
import type { PersistHandoffRequest as _codemind_v1_PersistHandoffRequest, PersistHandoffRequest__Output as _codemind_v1_PersistHandoffRequest__Output } from '../../codemind/v1/PersistHandoffRequest';
import type { PersistMemoryRequest as _codemind_v1_PersistMemoryRequest, PersistMemoryRequest__Output as _codemind_v1_PersistMemoryRequest__Output } from '../../codemind/v1/PersistMemoryRequest';
import type { PersistReviewContextRequest as _codemind_v1_PersistReviewContextRequest, PersistReviewContextRequest__Output as _codemind_v1_PersistReviewContextRequest__Output } from '../../codemind/v1/PersistReviewContextRequest';
import type { RetrieveHandoffRequest as _codemind_v1_RetrieveHandoffRequest, RetrieveHandoffRequest__Output as _codemind_v1_RetrieveHandoffRequest__Output } from '../../codemind/v1/RetrieveHandoffRequest';
import type { RetrieveMemoryRequest as _codemind_v1_RetrieveMemoryRequest, RetrieveMemoryRequest__Output as _codemind_v1_RetrieveMemoryRequest__Output } from '../../codemind/v1/RetrieveMemoryRequest';
import type { RetrieveMemoryResponse as _codemind_v1_RetrieveMemoryResponse, RetrieveMemoryResponse__Output as _codemind_v1_RetrieveMemoryResponse__Output } from '../../codemind/v1/RetrieveMemoryResponse';
import type { RetrieveReviewContextRequest as _codemind_v1_RetrieveReviewContextRequest, RetrieveReviewContextRequest__Output as _codemind_v1_RetrieveReviewContextRequest__Output } from '../../codemind/v1/RetrieveReviewContextRequest';
import type { TransitionHandoffRequest as _codemind_v1_TransitionHandoffRequest, TransitionHandoffRequest__Output as _codemind_v1_TransitionHandoffRequest__Output } from '../../codemind/v1/TransitionHandoffRequest';

export interface MemoryServiceClient extends grpc.Client {
  PersistHandoff(argument: _codemind_v1_PersistHandoffRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistHandoff(argument: _codemind_v1_PersistHandoffRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistHandoff(argument: _codemind_v1_PersistHandoffRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistHandoff(argument: _codemind_v1_PersistHandoffRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistHandoff(argument: _codemind_v1_PersistHandoffRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistHandoff(argument: _codemind_v1_PersistHandoffRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistHandoff(argument: _codemind_v1_PersistHandoffRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistHandoff(argument: _codemind_v1_PersistHandoffRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  
  PersistMemory(argument: _codemind_v1_PersistMemoryRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistMemory(argument: _codemind_v1_PersistMemoryRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistMemory(argument: _codemind_v1_PersistMemoryRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistMemory(argument: _codemind_v1_PersistMemoryRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistMemory(argument: _codemind_v1_PersistMemoryRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistMemory(argument: _codemind_v1_PersistMemoryRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistMemory(argument: _codemind_v1_PersistMemoryRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistMemory(argument: _codemind_v1_PersistMemoryRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  
  PersistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  PersistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  persistReviewContext(argument: _codemind_v1_PersistReviewContextRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  
  RetrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveHandoff(argument: _codemind_v1_RetrieveHandoffRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  
  RetrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveMemory(argument: _codemind_v1_RetrieveMemoryRequest, callback: grpc.requestCallback<_codemind_v1_RetrieveMemoryResponse__Output>): grpc.ClientUnaryCall;
  
  RetrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  RetrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  retrieveReviewContext(argument: _codemind_v1_RetrieveReviewContextRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  
  TransitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  TransitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  TransitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  TransitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  transitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  transitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  transitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  transitionHandoff(argument: _codemind_v1_TransitionHandoffRequest, callback: grpc.requestCallback<_codemind_v1_MemoryResponse__Output>): grpc.ClientUnaryCall;
  
}

export interface MemoryServiceHandlers extends grpc.UntypedServiceImplementation {
  PersistHandoff: grpc.handleUnaryCall<_codemind_v1_PersistHandoffRequest__Output, _codemind_v1_MemoryResponse>;
  
  PersistMemory: grpc.handleUnaryCall<_codemind_v1_PersistMemoryRequest__Output, _codemind_v1_MemoryResponse>;
  
  PersistReviewContext: grpc.handleUnaryCall<_codemind_v1_PersistReviewContextRequest__Output, _codemind_v1_MemoryResponse>;
  
  RetrieveHandoff: grpc.handleUnaryCall<_codemind_v1_RetrieveHandoffRequest__Output, _codemind_v1_MemoryResponse>;
  
  RetrieveMemory: grpc.handleUnaryCall<_codemind_v1_RetrieveMemoryRequest__Output, _codemind_v1_RetrieveMemoryResponse>;
  
  RetrieveReviewContext: grpc.handleUnaryCall<_codemind_v1_RetrieveReviewContextRequest__Output, _codemind_v1_MemoryResponse>;
  
  TransitionHandoff: grpc.handleUnaryCall<_codemind_v1_TransitionHandoffRequest__Output, _codemind_v1_MemoryResponse>;
  
}

export interface MemoryServiceDefinition extends grpc.ServiceDefinition {
  PersistHandoff: MethodDefinition<_codemind_v1_PersistHandoffRequest, _codemind_v1_MemoryResponse, _codemind_v1_PersistHandoffRequest__Output, _codemind_v1_MemoryResponse__Output>
  PersistMemory: MethodDefinition<_codemind_v1_PersistMemoryRequest, _codemind_v1_MemoryResponse, _codemind_v1_PersistMemoryRequest__Output, _codemind_v1_MemoryResponse__Output>
  PersistReviewContext: MethodDefinition<_codemind_v1_PersistReviewContextRequest, _codemind_v1_MemoryResponse, _codemind_v1_PersistReviewContextRequest__Output, _codemind_v1_MemoryResponse__Output>
  RetrieveHandoff: MethodDefinition<_codemind_v1_RetrieveHandoffRequest, _codemind_v1_MemoryResponse, _codemind_v1_RetrieveHandoffRequest__Output, _codemind_v1_MemoryResponse__Output>
  RetrieveMemory: MethodDefinition<_codemind_v1_RetrieveMemoryRequest, _codemind_v1_RetrieveMemoryResponse, _codemind_v1_RetrieveMemoryRequest__Output, _codemind_v1_RetrieveMemoryResponse__Output>
  RetrieveReviewContext: MethodDefinition<_codemind_v1_RetrieveReviewContextRequest, _codemind_v1_MemoryResponse, _codemind_v1_RetrieveReviewContextRequest__Output, _codemind_v1_MemoryResponse__Output>
  TransitionHandoff: MethodDefinition<_codemind_v1_TransitionHandoffRequest, _codemind_v1_MemoryResponse, _codemind_v1_TransitionHandoffRequest__Output, _codemind_v1_MemoryResponse__Output>
}
