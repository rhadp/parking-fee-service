// This is a generated file - do not edit.
//
// Generated from services/update_service.proto.

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

import 'update_service.pb.dart' as $0;

export 'update_service.pb.dart';

/// UpdateService provides adapter lifecycle management for the SDV system.
/// It handles installation, uninstallation, and state monitoring of parking operator adapters.
@$pb.GrpcServiceName('sdv.services.update.UpdateService')
class UpdateServiceClient extends $grpc.Client {
  /// The hostname for this service.
  static const $core.String defaultHost = '';

  /// OAuth scopes needed for the client.
  static const $core.List<$core.String> oauthScopes = [
    '',
  ];

  UpdateServiceClient(super.channel, {super.options, super.interceptors});

  /// Install an adapter from registry
  $grpc.ResponseFuture<$0.InstallAdapterResponse> installAdapter(
    $0.InstallAdapterRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$installAdapter, request, options: options);
  }

  /// Uninstall an adapter
  $grpc.ResponseFuture<$0.UninstallAdapterResponse> uninstallAdapter(
    $0.UninstallAdapterRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$uninstallAdapter, request, options: options);
  }

  /// List installed adapters
  $grpc.ResponseFuture<$0.ListAdaptersResponse> listAdapters(
    $0.ListAdaptersRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listAdapters, request, options: options);
  }

  /// Watch adapter state changes
  $grpc.ResponseStream<$0.AdapterStateEvent> watchAdapterStates(
    $0.WatchAdapterStatesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createStreamingCall(
        _$watchAdapterStates, $async.Stream.fromIterable([request]),
        options: options);
  }

  // method descriptors

  static final _$installAdapter =
      $grpc.ClientMethod<$0.InstallAdapterRequest, $0.InstallAdapterResponse>(
          '/sdv.services.update.UpdateService/InstallAdapter',
          ($0.InstallAdapterRequest value) => value.writeToBuffer(),
          $0.InstallAdapterResponse.fromBuffer);
  static final _$uninstallAdapter = $grpc.ClientMethod<
          $0.UninstallAdapterRequest, $0.UninstallAdapterResponse>(
      '/sdv.services.update.UpdateService/UninstallAdapter',
      ($0.UninstallAdapterRequest value) => value.writeToBuffer(),
      $0.UninstallAdapterResponse.fromBuffer);
  static final _$listAdapters =
      $grpc.ClientMethod<$0.ListAdaptersRequest, $0.ListAdaptersResponse>(
          '/sdv.services.update.UpdateService/ListAdapters',
          ($0.ListAdaptersRequest value) => value.writeToBuffer(),
          $0.ListAdaptersResponse.fromBuffer);
  static final _$watchAdapterStates =
      $grpc.ClientMethod<$0.WatchAdapterStatesRequest, $0.AdapterStateEvent>(
          '/sdv.services.update.UpdateService/WatchAdapterStates',
          ($0.WatchAdapterStatesRequest value) => value.writeToBuffer(),
          $0.AdapterStateEvent.fromBuffer);
}

@$pb.GrpcServiceName('sdv.services.update.UpdateService')
abstract class UpdateServiceBase extends $grpc.Service {
  $core.String get $name => 'sdv.services.update.UpdateService';

  UpdateServiceBase() {
    $addMethod($grpc.ServiceMethod<$0.InstallAdapterRequest,
            $0.InstallAdapterResponse>(
        'InstallAdapter',
        installAdapter_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.InstallAdapterRequest.fromBuffer(value),
        ($0.InstallAdapterResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.UninstallAdapterRequest,
            $0.UninstallAdapterResponse>(
        'UninstallAdapter',
        uninstallAdapter_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.UninstallAdapterRequest.fromBuffer(value),
        ($0.UninstallAdapterResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.ListAdaptersRequest, $0.ListAdaptersResponse>(
            'ListAdapters',
            listAdapters_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.ListAdaptersRequest.fromBuffer(value),
            ($0.ListAdaptersResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.WatchAdapterStatesRequest, $0.AdapterStateEvent>(
            'WatchAdapterStates',
            watchAdapterStates_Pre,
            false,
            true,
            ($core.List<$core.int> value) =>
                $0.WatchAdapterStatesRequest.fromBuffer(value),
            ($0.AdapterStateEvent value) => value.writeToBuffer()));
  }

  $async.Future<$0.InstallAdapterResponse> installAdapter_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.InstallAdapterRequest> $request) async {
    return installAdapter($call, await $request);
  }

  $async.Future<$0.InstallAdapterResponse> installAdapter(
      $grpc.ServiceCall call, $0.InstallAdapterRequest request);

  $async.Future<$0.UninstallAdapterResponse> uninstallAdapter_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.UninstallAdapterRequest> $request) async {
    return uninstallAdapter($call, await $request);
  }

  $async.Future<$0.UninstallAdapterResponse> uninstallAdapter(
      $grpc.ServiceCall call, $0.UninstallAdapterRequest request);

  $async.Future<$0.ListAdaptersResponse> listAdapters_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListAdaptersRequest> $request) async {
    return listAdapters($call, await $request);
  }

  $async.Future<$0.ListAdaptersResponse> listAdapters(
      $grpc.ServiceCall call, $0.ListAdaptersRequest request);

  $async.Stream<$0.AdapterStateEvent> watchAdapterStates_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.WatchAdapterStatesRequest> $request) async* {
    yield* watchAdapterStates($call, await $request);
  }

  $async.Stream<$0.AdapterStateEvent> watchAdapterStates(
      $grpc.ServiceCall call, $0.WatchAdapterStatesRequest request);
}
