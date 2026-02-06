// This is a generated file - do not edit.
//
// Generated from services/parking_adaptor.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:async' as $async;
import 'dart:core' as $core;

import 'package:grpc/service_api.dart' as $grpc;
import 'package:protobuf/protobuf.dart' as $pb;

import 'parking_adaptor.pb.dart' as $0;

export 'parking_adaptor.pb.dart';

/// ParkingAdaptor service provides parking session management.
/// Called by PARKING_APP via gRPC/TLS for manual session control and status.
@$pb.GrpcServiceName('sdv.services.parking.ParkingAdaptor')
class ParkingAdaptorClient extends $grpc.Client {
  /// The hostname for this service.
  static const $core.String defaultHost = '';

  /// OAuth scopes needed for the client.
  static const $core.List<$core.String> oauthScopes = [
    '',
  ];

  ParkingAdaptorClient(super.channel, {super.options, super.interceptors});

  /// Start a new parking session.
  /// Zone_ID is provided by PARKING_APP (obtained from PARKING_FEE_SERVICE).
  $grpc.ResponseFuture<$0.StartSessionResponse> startSession(
    $0.StartSessionRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$startSession, request, options: options);
  }

  /// Stop the current parking session.
  $grpc.ResponseFuture<$0.StopSessionResponse> stopSession(
    $0.StopSessionRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$stopSession, request, options: options);
  }

  /// Get the current session status.
  $grpc.ResponseFuture<$0.GetSessionStatusResponse> getSessionStatus(
    $0.GetSessionStatusRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getSessionStatus, request, options: options);
  }

  // method descriptors

  static final _$startSession =
      $grpc.ClientMethod<$0.StartSessionRequest, $0.StartSessionResponse>(
          '/sdv.services.parking.ParkingAdaptor/StartSession',
          ($0.StartSessionRequest value) => value.writeToBuffer(),
          $0.StartSessionResponse.fromBuffer);
  static final _$stopSession =
      $grpc.ClientMethod<$0.StopSessionRequest, $0.StopSessionResponse>(
          '/sdv.services.parking.ParkingAdaptor/StopSession',
          ($0.StopSessionRequest value) => value.writeToBuffer(),
          $0.StopSessionResponse.fromBuffer);
  static final _$getSessionStatus = $grpc.ClientMethod<
          $0.GetSessionStatusRequest, $0.GetSessionStatusResponse>(
      '/sdv.services.parking.ParkingAdaptor/GetSessionStatus',
      ($0.GetSessionStatusRequest value) => value.writeToBuffer(),
      $0.GetSessionStatusResponse.fromBuffer);
}

@$pb.GrpcServiceName('sdv.services.parking.ParkingAdaptor')
abstract class ParkingAdaptorServiceBase extends $grpc.Service {
  $core.String get $name => 'sdv.services.parking.ParkingAdaptor';

  ParkingAdaptorServiceBase() {
    $addMethod(
        $grpc.ServiceMethod<$0.StartSessionRequest, $0.StartSessionResponse>(
            'StartSession',
            startSession_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.StartSessionRequest.fromBuffer(value),
            ($0.StartSessionResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.StopSessionRequest, $0.StopSessionResponse>(
            'StopSession',
            stopSession_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.StopSessionRequest.fromBuffer(value),
            ($0.StopSessionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetSessionStatusRequest,
            $0.GetSessionStatusResponse>(
        'GetSessionStatus',
        getSessionStatus_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.GetSessionStatusRequest.fromBuffer(value),
        ($0.GetSessionStatusResponse value) => value.writeToBuffer()));
  }

  $async.Future<$0.StartSessionResponse> startSession_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.StartSessionRequest> $request) async {
    return startSession($call, await $request);
  }

  $async.Future<$0.StartSessionResponse> startSession(
      $grpc.ServiceCall call, $0.StartSessionRequest request);

  $async.Future<$0.StopSessionResponse> stopSession_Pre($grpc.ServiceCall $call,
      $async.Future<$0.StopSessionRequest> $request) async {
    return stopSession($call, await $request);
  }

  $async.Future<$0.StopSessionResponse> stopSession(
      $grpc.ServiceCall call, $0.StopSessionRequest request);

  $async.Future<$0.GetSessionStatusResponse> getSessionStatus_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.GetSessionStatusRequest> $request) async {
    return getSessionStatus($call, await $request);
  }

  $async.Future<$0.GetSessionStatusResponse> getSessionStatus(
      $grpc.ServiceCall call, $0.GetSessionStatusRequest request);
}
