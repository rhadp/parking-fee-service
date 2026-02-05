// This is a generated file - do not edit.
//
// Generated from services/databroker.proto.

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

import 'databroker.pb.dart' as $0;

export 'databroker.pb.dart';

/// DataBroker provides a VSS-compliant gRPC pub/sub interface for vehicle signals.
/// This service is compatible with Eclipse Kuksa Databroker.
@$pb.GrpcServiceName('sdv.services.databroker.DataBroker')
class DataBrokerClient extends $grpc.Client {
  /// The hostname for this service.
  static const $core.String defaultHost = '';

  /// OAuth scopes needed for the client.
  static const $core.List<$core.String> oauthScopes = [
    '',
  ];

  DataBrokerClient(super.channel, {super.options, super.interceptors});

  /// Get current value of one or more signals
  $grpc.ResponseFuture<$0.GetSignalResponse> getSignal(
    $0.GetSignalRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getSignal, request, options: options);
  }

  /// Set a signal value (write access required)
  $grpc.ResponseFuture<$0.SetSignalResponse> setSignal(
    $0.SetSignalRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$setSignal, request, options: options);
  }

  /// Subscribe to signal changes (server-side streaming)
  $grpc.ResponseStream<$0.SubscribeResponse> subscribe(
    $0.SubscribeRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createStreamingCall(
        _$subscribe, $async.Stream.fromIterable([request]),
        options: options);
  }

  // method descriptors

  static final _$getSignal =
      $grpc.ClientMethod<$0.GetSignalRequest, $0.GetSignalResponse>(
          '/sdv.services.databroker.DataBroker/GetSignal',
          ($0.GetSignalRequest value) => value.writeToBuffer(),
          $0.GetSignalResponse.fromBuffer);
  static final _$setSignal =
      $grpc.ClientMethod<$0.SetSignalRequest, $0.SetSignalResponse>(
          '/sdv.services.databroker.DataBroker/SetSignal',
          ($0.SetSignalRequest value) => value.writeToBuffer(),
          $0.SetSignalResponse.fromBuffer);
  static final _$subscribe =
      $grpc.ClientMethod<$0.SubscribeRequest, $0.SubscribeResponse>(
          '/sdv.services.databroker.DataBroker/Subscribe',
          ($0.SubscribeRequest value) => value.writeToBuffer(),
          $0.SubscribeResponse.fromBuffer);
}

@$pb.GrpcServiceName('sdv.services.databroker.DataBroker')
abstract class DataBrokerServiceBase extends $grpc.Service {
  $core.String get $name => 'sdv.services.databroker.DataBroker';

  DataBrokerServiceBase() {
    $addMethod($grpc.ServiceMethod<$0.GetSignalRequest, $0.GetSignalResponse>(
        'GetSignal',
        getSignal_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.GetSignalRequest.fromBuffer(value),
        ($0.GetSignalResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.SetSignalRequest, $0.SetSignalResponse>(
        'SetSignal',
        setSignal_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.SetSignalRequest.fromBuffer(value),
        ($0.SetSignalResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.SubscribeRequest, $0.SubscribeResponse>(
        'Subscribe',
        subscribe_Pre,
        false,
        true,
        ($core.List<$core.int> value) => $0.SubscribeRequest.fromBuffer(value),
        ($0.SubscribeResponse value) => value.writeToBuffer()));
  }

  $async.Future<$0.GetSignalResponse> getSignal_Pre($grpc.ServiceCall $call,
      $async.Future<$0.GetSignalRequest> $request) async {
    return getSignal($call, await $request);
  }

  $async.Future<$0.GetSignalResponse> getSignal(
      $grpc.ServiceCall call, $0.GetSignalRequest request);

  $async.Future<$0.SetSignalResponse> setSignal_Pre($grpc.ServiceCall $call,
      $async.Future<$0.SetSignalRequest> $request) async {
    return setSignal($call, await $request);
  }

  $async.Future<$0.SetSignalResponse> setSignal(
      $grpc.ServiceCall call, $0.SetSignalRequest request);

  $async.Stream<$0.SubscribeResponse> subscribe_Pre($grpc.ServiceCall $call,
      $async.Future<$0.SubscribeRequest> $request) async* {
    yield* subscribe($call, await $request);
  }

  $async.Stream<$0.SubscribeResponse> subscribe(
      $grpc.ServiceCall call, $0.SubscribeRequest request);
}
