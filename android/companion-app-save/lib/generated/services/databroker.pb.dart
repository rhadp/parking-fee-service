// This is a generated file - do not edit.
//
// Generated from services/databroker.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

import '../vss/signals.pb.dart' as $1;

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

/// Request to get current values of signals
class GetSignalRequest extends $pb.GeneratedMessage {
  factory GetSignalRequest({
    $core.Iterable<$core.String>? signalPaths,
  }) {
    final result = create();
    if (signalPaths != null) result.signalPaths.addAll(signalPaths);
    return result;
  }

  GetSignalRequest._();

  factory GetSignalRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetSignalRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetSignalRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.databroker'),
      createEmptyInstance: create)
    ..pPS(1, _omitFieldNames ? '' : 'signalPaths')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSignalRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSignalRequest copyWith(void Function(GetSignalRequest) updates) =>
      super.copyWith((message) => updates(message as GetSignalRequest))
          as GetSignalRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetSignalRequest create() => GetSignalRequest._();
  @$core.override
  GetSignalRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetSignalRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetSignalRequest>(create);
  static GetSignalRequest? _defaultInstance;

  /// VSS signal paths to retrieve (e.g., "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get signalPaths => $_getList(0);
}

/// Response containing current signal values
class GetSignalResponse extends $pb.GeneratedMessage {
  factory GetSignalResponse({
    $core.Iterable<$1.VehicleSignal>? signals,
  }) {
    final result = create();
    if (signals != null) result.signals.addAll(signals);
    return result;
  }

  GetSignalResponse._();

  factory GetSignalResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetSignalResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetSignalResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.databroker'),
      createEmptyInstance: create)
    ..pPM<$1.VehicleSignal>(1, _omitFieldNames ? '' : 'signals',
        subBuilder: $1.VehicleSignal.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSignalResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSignalResponse copyWith(void Function(GetSignalResponse) updates) =>
      super.copyWith((message) => updates(message as GetSignalResponse))
          as GetSignalResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetSignalResponse create() => GetSignalResponse._();
  @$core.override
  GetSignalResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetSignalResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetSignalResponse>(create);
  static GetSignalResponse? _defaultInstance;

  /// Current values of requested signals
  @$pb.TagNumber(1)
  $pb.PbList<$1.VehicleSignal> get signals => $_getList(0);
}

/// Request to set a signal value
class SetSignalRequest extends $pb.GeneratedMessage {
  factory SetSignalRequest({
    $core.String? signalPath,
    $1.VehicleSignal? signal,
  }) {
    final result = create();
    if (signalPath != null) result.signalPath = signalPath;
    if (signal != null) result.signal = signal;
    return result;
  }

  SetSignalRequest._();

  factory SetSignalRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory SetSignalRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SetSignalRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.databroker'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'signalPath')
    ..aOM<$1.VehicleSignal>(2, _omitFieldNames ? '' : 'signal',
        subBuilder: $1.VehicleSignal.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SetSignalRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SetSignalRequest copyWith(void Function(SetSignalRequest) updates) =>
      super.copyWith((message) => updates(message as SetSignalRequest))
          as SetSignalRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static SetSignalRequest create() => SetSignalRequest._();
  @$core.override
  SetSignalRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static SetSignalRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SetSignalRequest>(create);
  static SetSignalRequest? _defaultInstance;

  /// VSS signal path to set (e.g., "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
  @$pb.TagNumber(1)
  $core.String get signalPath => $_getSZ(0);
  @$pb.TagNumber(1)
  set signalPath($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSignalPath() => $_has(0);
  @$pb.TagNumber(1)
  void clearSignalPath() => $_clearField(1);

  /// Signal value to set
  @$pb.TagNumber(2)
  $1.VehicleSignal get signal => $_getN(1);
  @$pb.TagNumber(2)
  set signal($1.VehicleSignal value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasSignal() => $_has(1);
  @$pb.TagNumber(2)
  void clearSignal() => $_clearField(2);
  @$pb.TagNumber(2)
  $1.VehicleSignal ensureSignal() => $_ensure(1);
}

/// Response for set signal operation
class SetSignalResponse extends $pb.GeneratedMessage {
  factory SetSignalResponse({
    $core.bool? success,
    $core.String? errorMessage,
  }) {
    final result = create();
    if (success != null) result.success = success;
    if (errorMessage != null) result.errorMessage = errorMessage;
    return result;
  }

  SetSignalResponse._();

  factory SetSignalResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory SetSignalResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SetSignalResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.databroker'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'success')
    ..aOS(2, _omitFieldNames ? '' : 'errorMessage')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SetSignalResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SetSignalResponse copyWith(void Function(SetSignalResponse) updates) =>
      super.copyWith((message) => updates(message as SetSignalResponse))
          as SetSignalResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static SetSignalResponse create() => SetSignalResponse._();
  @$core.override
  SetSignalResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static SetSignalResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SetSignalResponse>(create);
  static SetSignalResponse? _defaultInstance;

  /// Whether the operation succeeded
  @$pb.TagNumber(1)
  $core.bool get success => $_getBF(0);
  @$pb.TagNumber(1)
  set success($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSuccess() => $_has(0);
  @$pb.TagNumber(1)
  void clearSuccess() => $_clearField(1);

  /// Error message if operation failed
  @$pb.TagNumber(2)
  $core.String get errorMessage => $_getSZ(1);
  @$pb.TagNumber(2)
  set errorMessage($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasErrorMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearErrorMessage() => $_clearField(2);
}

/// Request to subscribe to signal changes
class SubscribeRequest extends $pb.GeneratedMessage {
  factory SubscribeRequest({
    $core.Iterable<$core.String>? signalPaths,
  }) {
    final result = create();
    if (signalPaths != null) result.signalPaths.addAll(signalPaths);
    return result;
  }

  SubscribeRequest._();

  factory SubscribeRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory SubscribeRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SubscribeRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.databroker'),
      createEmptyInstance: create)
    ..pPS(1, _omitFieldNames ? '' : 'signalPaths')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SubscribeRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SubscribeRequest copyWith(void Function(SubscribeRequest) updates) =>
      super.copyWith((message) => updates(message as SubscribeRequest))
          as SubscribeRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static SubscribeRequest create() => SubscribeRequest._();
  @$core.override
  SubscribeRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static SubscribeRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SubscribeRequest>(create);
  static SubscribeRequest? _defaultInstance;

  /// VSS signal paths to subscribe to
  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get signalPaths => $_getList(0);
}

/// Streamed response for signal changes
class SubscribeResponse extends $pb.GeneratedMessage {
  factory SubscribeResponse({
    $core.String? signalPath,
    $1.VehicleSignal? signal,
  }) {
    final result = create();
    if (signalPath != null) result.signalPath = signalPath;
    if (signal != null) result.signal = signal;
    return result;
  }

  SubscribeResponse._();

  factory SubscribeResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory SubscribeResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SubscribeResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.databroker'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'signalPath')
    ..aOM<$1.VehicleSignal>(2, _omitFieldNames ? '' : 'signal',
        subBuilder: $1.VehicleSignal.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SubscribeResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SubscribeResponse copyWith(void Function(SubscribeResponse) updates) =>
      super.copyWith((message) => updates(message as SubscribeResponse))
          as SubscribeResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static SubscribeResponse create() => SubscribeResponse._();
  @$core.override
  SubscribeResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static SubscribeResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SubscribeResponse>(create);
  static SubscribeResponse? _defaultInstance;

  /// VSS signal path that changed
  @$pb.TagNumber(1)
  $core.String get signalPath => $_getSZ(0);
  @$pb.TagNumber(1)
  set signalPath($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSignalPath() => $_has(0);
  @$pb.TagNumber(1)
  void clearSignalPath() => $_clearField(1);

  /// Updated signal value
  @$pb.TagNumber(2)
  $1.VehicleSignal get signal => $_getN(1);
  @$pb.TagNumber(2)
  set signal($1.VehicleSignal value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasSignal() => $_has(1);
  @$pb.TagNumber(2)
  void clearSignal() => $_clearField(2);
  @$pb.TagNumber(2)
  $1.VehicleSignal ensureSignal() => $_ensure(1);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
