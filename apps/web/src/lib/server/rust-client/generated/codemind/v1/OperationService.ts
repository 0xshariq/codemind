// Original file: ../../proto/v1/operation.proto

import type * as grpc from '@grpc/grpc-js'
import type { MethodDefinition } from '@grpc/proto-loader'
import type { ProgressEvent as _codemind_v1_ProgressEvent, ProgressEvent__Output as _codemind_v1_ProgressEvent__Output } from '../../codemind/v1/ProgressEvent';
import type { StartInvestigationRequest as _codemind_v1_StartInvestigationRequest, StartInvestigationRequest__Output as _codemind_v1_StartInvestigationRequest__Output } from '../../codemind/v1/StartInvestigationRequest';
import type { StartInvestigationResponse as _codemind_v1_StartInvestigationResponse, StartInvestigationResponse__Output as _codemind_v1_StartInvestigationResponse__Output } from '../../codemind/v1/StartInvestigationResponse';
import type { StreamProgressRequest as _codemind_v1_StreamProgressRequest, StreamProgressRequest__Output as _codemind_v1_StreamProgressRequest__Output } from '../../codemind/v1/StreamProgressRequest';

export interface OperationServiceClient extends grpc.Client {
  StartInvestigation(argument: _codemind_v1_StartInvestigationRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  StartInvestigation(argument: _codemind_v1_StartInvestigationRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  StartInvestigation(argument: _codemind_v1_StartInvestigationRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  StartInvestigation(argument: _codemind_v1_StartInvestigationRequest, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  startInvestigation(argument: _codemind_v1_StartInvestigationRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  startInvestigation(argument: _codemind_v1_StartInvestigationRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  startInvestigation(argument: _codemind_v1_StartInvestigationRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  startInvestigation(argument: _codemind_v1_StartInvestigationRequest, callback: grpc.requestCallback<_codemind_v1_StartInvestigationResponse__Output>): grpc.ClientUnaryCall;
  
  StreamProgress(argument: _codemind_v1_StreamProgressRequest, metadata: grpc.Metadata, options?: grpc.CallOptions): grpc.ClientReadableStream<_codemind_v1_ProgressEvent__Output>;
  StreamProgress(argument: _codemind_v1_StreamProgressRequest, options?: grpc.CallOptions): grpc.ClientReadableStream<_codemind_v1_ProgressEvent__Output>;
  streamProgress(argument: _codemind_v1_StreamProgressRequest, metadata: grpc.Metadata, options?: grpc.CallOptions): grpc.ClientReadableStream<_codemind_v1_ProgressEvent__Output>;
  streamProgress(argument: _codemind_v1_StreamProgressRequest, options?: grpc.CallOptions): grpc.ClientReadableStream<_codemind_v1_ProgressEvent__Output>;
  
}

export interface OperationServiceHandlers extends grpc.UntypedServiceImplementation {
  StartInvestigation: grpc.handleUnaryCall<_codemind_v1_StartInvestigationRequest__Output, _codemind_v1_StartInvestigationResponse>;
  
  StreamProgress: grpc.handleServerStreamingCall<_codemind_v1_StreamProgressRequest__Output, _codemind_v1_ProgressEvent>;
  
}

export interface OperationServiceDefinition extends grpc.ServiceDefinition {
  StartInvestigation: MethodDefinition<_codemind_v1_StartInvestigationRequest, _codemind_v1_StartInvestigationResponse, _codemind_v1_StartInvestigationRequest__Output, _codemind_v1_StartInvestigationResponse__Output>
  StreamProgress: MethodDefinition<_codemind_v1_StreamProgressRequest, _codemind_v1_ProgressEvent, _codemind_v1_StreamProgressRequest__Output, _codemind_v1_ProgressEvent__Output>
}
