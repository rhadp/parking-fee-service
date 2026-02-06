// This is a generated file - do not edit.
//
// Generated from services/parking_adaptor.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:fixnum/fixnum.dart' as $fixnum;
import 'package:protobuf/protobuf.dart' as $pb;

import 'parking_adaptor.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'parking_adaptor.pbenum.dart';

/// Request to start a parking session.
class StartSessionRequest extends $pb.GeneratedMessage {
  factory StartSessionRequest({
    $core.String? zoneId,
  }) {
    final result = create();
    if (zoneId != null) result.zoneId = zoneId;
    return result;
  }

  StartSessionRequest._();

  factory StartSessionRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory StartSessionRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StartSessionRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.parking'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'zoneId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StartSessionRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StartSessionRequest copyWith(void Function(StartSessionRequest) updates) =>
      super.copyWith((message) => updates(message as StartSessionRequest))
          as StartSessionRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static StartSessionRequest create() => StartSessionRequest._();
  @$core.override
  StartSessionRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static StartSessionRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StartSessionRequest>(create);
  static StartSessionRequest? _defaultInstance;

  /// Zone_ID provided by PARKING_APP (obtained from PARKING_FEE_SERVICE based on location)
  @$pb.TagNumber(1)
  $core.String get zoneId => $_getSZ(0);
  @$pb.TagNumber(1)
  set zoneId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasZoneId() => $_has(0);
  @$pb.TagNumber(1)
  void clearZoneId() => $_clearField(1);
}

/// Response from starting a parking session.
class StartSessionResponse extends $pb.GeneratedMessage {
  factory StartSessionResponse({
    $core.bool? success,
    $core.String? errorMessage,
    $core.String? sessionId,
    SessionState? state,
  }) {
    final result = create();
    if (success != null) result.success = success;
    if (errorMessage != null) result.errorMessage = errorMessage;
    if (sessionId != null) result.sessionId = sessionId;
    if (state != null) result.state = state;
    return result;
  }

  StartSessionResponse._();

  factory StartSessionResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory StartSessionResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StartSessionResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.parking'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'success')
    ..aOS(2, _omitFieldNames ? '' : 'errorMessage')
    ..aOS(3, _omitFieldNames ? '' : 'sessionId')
    ..aE<SessionState>(4, _omitFieldNames ? '' : 'state',
        enumValues: SessionState.values)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StartSessionResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StartSessionResponse copyWith(void Function(StartSessionResponse) updates) =>
      super.copyWith((message) => updates(message as StartSessionResponse))
          as StartSessionResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static StartSessionResponse create() => StartSessionResponse._();
  @$core.override
  StartSessionResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static StartSessionResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StartSessionResponse>(create);
  static StartSessionResponse? _defaultInstance;

  /// Whether the operation succeeded
  @$pb.TagNumber(1)
  $core.bool get success => $_getBF(0);
  @$pb.TagNumber(1)
  set success($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSuccess() => $_has(0);
  @$pb.TagNumber(1)
  void clearSuccess() => $_clearField(1);

  /// Error message if failed
  @$pb.TagNumber(2)
  $core.String get errorMessage => $_getSZ(1);
  @$pb.TagNumber(2)
  set errorMessage($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasErrorMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearErrorMessage() => $_clearField(2);

  /// Session ID from PARKING_OPERATOR
  @$pb.TagNumber(3)
  $core.String get sessionId => $_getSZ(2);
  @$pb.TagNumber(3)
  set sessionId($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasSessionId() => $_has(2);
  @$pb.TagNumber(3)
  void clearSessionId() => $_clearField(3);

  /// Current session state
  @$pb.TagNumber(4)
  SessionState get state => $_getN(3);
  @$pb.TagNumber(4)
  set state(SessionState value) => $_setField(4, value);
  @$pb.TagNumber(4)
  $core.bool hasState() => $_has(3);
  @$pb.TagNumber(4)
  void clearState() => $_clearField(4);
}

/// Request to stop the current parking session.
class StopSessionRequest extends $pb.GeneratedMessage {
  factory StopSessionRequest() => create();

  StopSessionRequest._();

  factory StopSessionRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory StopSessionRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StopSessionRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.parking'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StopSessionRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StopSessionRequest copyWith(void Function(StopSessionRequest) updates) =>
      super.copyWith((message) => updates(message as StopSessionRequest))
          as StopSessionRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static StopSessionRequest create() => StopSessionRequest._();
  @$core.override
  StopSessionRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static StopSessionRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StopSessionRequest>(create);
  static StopSessionRequest? _defaultInstance;
}

/// Response from stopping a parking session.
class StopSessionResponse extends $pb.GeneratedMessage {
  factory StopSessionResponse({
    $core.bool? success,
    $core.String? errorMessage,
    $core.String? sessionId,
    SessionState? state,
    $core.double? finalCost,
    $fixnum.Int64? durationSeconds,
  }) {
    final result = create();
    if (success != null) result.success = success;
    if (errorMessage != null) result.errorMessage = errorMessage;
    if (sessionId != null) result.sessionId = sessionId;
    if (state != null) result.state = state;
    if (finalCost != null) result.finalCost = finalCost;
    if (durationSeconds != null) result.durationSeconds = durationSeconds;
    return result;
  }

  StopSessionResponse._();

  factory StopSessionResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory StopSessionResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StopSessionResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.parking'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'success')
    ..aOS(2, _omitFieldNames ? '' : 'errorMessage')
    ..aOS(3, _omitFieldNames ? '' : 'sessionId')
    ..aE<SessionState>(4, _omitFieldNames ? '' : 'state',
        enumValues: SessionState.values)
    ..aD(5, _omitFieldNames ? '' : 'finalCost')
    ..aInt64(6, _omitFieldNames ? '' : 'durationSeconds')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StopSessionResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StopSessionResponse copyWith(void Function(StopSessionResponse) updates) =>
      super.copyWith((message) => updates(message as StopSessionResponse))
          as StopSessionResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static StopSessionResponse create() => StopSessionResponse._();
  @$core.override
  StopSessionResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static StopSessionResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StopSessionResponse>(create);
  static StopSessionResponse? _defaultInstance;

  /// Whether the operation succeeded
  @$pb.TagNumber(1)
  $core.bool get success => $_getBF(0);
  @$pb.TagNumber(1)
  set success($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSuccess() => $_has(0);
  @$pb.TagNumber(1)
  void clearSuccess() => $_clearField(1);

  /// Error message if failed
  @$pb.TagNumber(2)
  $core.String get errorMessage => $_getSZ(1);
  @$pb.TagNumber(2)
  set errorMessage($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasErrorMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearErrorMessage() => $_clearField(2);

  /// Session ID that was stopped
  @$pb.TagNumber(3)
  $core.String get sessionId => $_getSZ(2);
  @$pb.TagNumber(3)
  set sessionId($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasSessionId() => $_has(2);
  @$pb.TagNumber(3)
  void clearSessionId() => $_clearField(3);

  /// Current session state
  @$pb.TagNumber(4)
  SessionState get state => $_getN(3);
  @$pb.TagNumber(4)
  set state(SessionState value) => $_setField(4, value);
  @$pb.TagNumber(4)
  $core.bool hasState() => $_has(3);
  @$pb.TagNumber(4)
  void clearState() => $_clearField(4);

  /// Final cost of the session
  @$pb.TagNumber(5)
  $core.double get finalCost => $_getN(4);
  @$pb.TagNumber(5)
  set finalCost($core.double value) => $_setDouble(4, value);
  @$pb.TagNumber(5)
  $core.bool hasFinalCost() => $_has(4);
  @$pb.TagNumber(5)
  void clearFinalCost() => $_clearField(5);

  /// Duration of the session in seconds
  @$pb.TagNumber(6)
  $fixnum.Int64 get durationSeconds => $_getI64(5);
  @$pb.TagNumber(6)
  set durationSeconds($fixnum.Int64 value) => $_setInt64(5, value);
  @$pb.TagNumber(6)
  $core.bool hasDurationSeconds() => $_has(5);
  @$pb.TagNumber(6)
  void clearDurationSeconds() => $_clearField(6);
}

/// Request to get session status.
class GetSessionStatusRequest extends $pb.GeneratedMessage {
  factory GetSessionStatusRequest() => create();

  GetSessionStatusRequest._();

  factory GetSessionStatusRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetSessionStatusRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetSessionStatusRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.parking'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSessionStatusRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSessionStatusRequest copyWith(
          void Function(GetSessionStatusRequest) updates) =>
      super.copyWith((message) => updates(message as GetSessionStatusRequest))
          as GetSessionStatusRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetSessionStatusRequest create() => GetSessionStatusRequest._();
  @$core.override
  GetSessionStatusRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetSessionStatusRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetSessionStatusRequest>(create);
  static GetSessionStatusRequest? _defaultInstance;
}

/// Response with session status information.
class GetSessionStatusResponse extends $pb.GeneratedMessage {
  factory GetSessionStatusResponse({
    $core.bool? hasActiveSession,
    $core.String? sessionId,
    SessionState? state,
    $fixnum.Int64? startTimeUnix,
    $fixnum.Int64? durationSeconds,
    $core.double? currentCost,
    $core.String? zoneId,
    $core.String? errorMessage,
    $core.double? latitude,
    $core.double? longitude,
  }) {
    final result = create();
    if (hasActiveSession != null) result.hasActiveSession = hasActiveSession;
    if (sessionId != null) result.sessionId = sessionId;
    if (state != null) result.state = state;
    if (startTimeUnix != null) result.startTimeUnix = startTimeUnix;
    if (durationSeconds != null) result.durationSeconds = durationSeconds;
    if (currentCost != null) result.currentCost = currentCost;
    if (zoneId != null) result.zoneId = zoneId;
    if (errorMessage != null) result.errorMessage = errorMessage;
    if (latitude != null) result.latitude = latitude;
    if (longitude != null) result.longitude = longitude;
    return result;
  }

  GetSessionStatusResponse._();

  factory GetSessionStatusResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetSessionStatusResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetSessionStatusResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.parking'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'hasActiveSession')
    ..aOS(2, _omitFieldNames ? '' : 'sessionId')
    ..aE<SessionState>(3, _omitFieldNames ? '' : 'state',
        enumValues: SessionState.values)
    ..aInt64(4, _omitFieldNames ? '' : 'startTimeUnix')
    ..aInt64(5, _omitFieldNames ? '' : 'durationSeconds')
    ..aD(6, _omitFieldNames ? '' : 'currentCost')
    ..aOS(7, _omitFieldNames ? '' : 'zoneId')
    ..aOS(8, _omitFieldNames ? '' : 'errorMessage')
    ..aD(9, _omitFieldNames ? '' : 'latitude')
    ..aD(10, _omitFieldNames ? '' : 'longitude')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSessionStatusResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSessionStatusResponse copyWith(
          void Function(GetSessionStatusResponse) updates) =>
      super.copyWith((message) => updates(message as GetSessionStatusResponse))
          as GetSessionStatusResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetSessionStatusResponse create() => GetSessionStatusResponse._();
  @$core.override
  GetSessionStatusResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetSessionStatusResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetSessionStatusResponse>(create);
  static GetSessionStatusResponse? _defaultInstance;

  /// Whether there is an active session
  @$pb.TagNumber(1)
  $core.bool get hasActiveSession => $_getBF(0);
  @$pb.TagNumber(1)
  set hasActiveSession($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasHasActiveSession() => $_has(0);
  @$pb.TagNumber(1)
  void clearHasActiveSession() => $_clearField(1);

  /// Session ID (empty if no session)
  @$pb.TagNumber(2)
  $core.String get sessionId => $_getSZ(1);
  @$pb.TagNumber(2)
  set sessionId($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasSessionId() => $_has(1);
  @$pb.TagNumber(2)
  void clearSessionId() => $_clearField(2);

  /// Current session state
  @$pb.TagNumber(3)
  SessionState get state => $_getN(2);
  @$pb.TagNumber(3)
  set state(SessionState value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasState() => $_has(2);
  @$pb.TagNumber(3)
  void clearState() => $_clearField(3);

  /// Session start time as Unix timestamp
  @$pb.TagNumber(4)
  $fixnum.Int64 get startTimeUnix => $_getI64(3);
  @$pb.TagNumber(4)
  set startTimeUnix($fixnum.Int64 value) => $_setInt64(3, value);
  @$pb.TagNumber(4)
  $core.bool hasStartTimeUnix() => $_has(3);
  @$pb.TagNumber(4)
  void clearStartTimeUnix() => $_clearField(4);

  /// Duration of the session in seconds
  @$pb.TagNumber(5)
  $fixnum.Int64 get durationSeconds => $_getI64(4);
  @$pb.TagNumber(5)
  set durationSeconds($fixnum.Int64 value) => $_setInt64(4, value);
  @$pb.TagNumber(5)
  $core.bool hasDurationSeconds() => $_has(4);
  @$pb.TagNumber(5)
  void clearDurationSeconds() => $_clearField(5);

  /// Current cost of the session
  @$pb.TagNumber(6)
  $core.double get currentCost => $_getN(5);
  @$pb.TagNumber(6)
  set currentCost($core.double value) => $_setDouble(5, value);
  @$pb.TagNumber(6)
  $core.bool hasCurrentCost() => $_has(5);
  @$pb.TagNumber(6)
  void clearCurrentCost() => $_clearField(6);

  /// Zone ID where parked
  @$pb.TagNumber(7)
  $core.String get zoneId => $_getSZ(6);
  @$pb.TagNumber(7)
  set zoneId($core.String value) => $_setString(6, value);
  @$pb.TagNumber(7)
  $core.bool hasZoneId() => $_has(6);
  @$pb.TagNumber(7)
  void clearZoneId() => $_clearField(7);

  /// Error message if state is ERROR
  @$pb.TagNumber(8)
  $core.String get errorMessage => $_getSZ(7);
  @$pb.TagNumber(8)
  set errorMessage($core.String value) => $_setString(7, value);
  @$pb.TagNumber(8)
  $core.bool hasErrorMessage() => $_has(7);
  @$pb.TagNumber(8)
  void clearErrorMessage() => $_clearField(8);

  /// Latitude where session started
  @$pb.TagNumber(9)
  $core.double get latitude => $_getN(8);
  @$pb.TagNumber(9)
  set latitude($core.double value) => $_setDouble(8, value);
  @$pb.TagNumber(9)
  $core.bool hasLatitude() => $_has(8);
  @$pb.TagNumber(9)
  void clearLatitude() => $_clearField(9);

  /// Longitude where session started
  @$pb.TagNumber(10)
  $core.double get longitude => $_getN(9);
  @$pb.TagNumber(10)
  set longitude($core.double value) => $_setDouble(9, value);
  @$pb.TagNumber(10)
  $core.bool hasLongitude() => $_has(9);
  @$pb.TagNumber(10)
  void clearLongitude() => $_clearField(10);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
