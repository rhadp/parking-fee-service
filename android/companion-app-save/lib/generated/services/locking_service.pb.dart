// This is a generated file - do not edit.
//
// Generated from services/locking_service.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

import 'locking_service.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'locking_service.pbenum.dart';

class LockRequest extends $pb.GeneratedMessage {
  factory LockRequest({
    Door? door,
    $core.String? commandId,
    $core.String? authToken,
  }) {
    final result = create();
    if (door != null) result.door = door;
    if (commandId != null) result.commandId = commandId;
    if (authToken != null) result.authToken = authToken;
    return result;
  }

  LockRequest._();

  factory LockRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory LockRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'LockRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.locking'),
      createEmptyInstance: create)
    ..aE<Door>(1, _omitFieldNames ? '' : 'door', enumValues: Door.values)
    ..aOS(2, _omitFieldNames ? '' : 'commandId')
    ..aOS(3, _omitFieldNames ? '' : 'authToken')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  LockRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  LockRequest copyWith(void Function(LockRequest) updates) =>
      super.copyWith((message) => updates(message as LockRequest))
          as LockRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static LockRequest create() => LockRequest._();
  @$core.override
  LockRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static LockRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<LockRequest>(create);
  static LockRequest? _defaultInstance;

  @$pb.TagNumber(1)
  Door get door => $_getN(0);
  @$pb.TagNumber(1)
  set door(Door value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasDoor() => $_has(0);
  @$pb.TagNumber(1)
  void clearDoor() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get commandId => $_getSZ(1);
  @$pb.TagNumber(2)
  set commandId($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasCommandId() => $_has(1);
  @$pb.TagNumber(2)
  void clearCommandId() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get authToken => $_getSZ(2);
  @$pb.TagNumber(3)
  set authToken($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasAuthToken() => $_has(2);
  @$pb.TagNumber(3)
  void clearAuthToken() => $_clearField(3);
}

class LockResponse extends $pb.GeneratedMessage {
  factory LockResponse({
    $core.bool? success,
    $core.String? errorMessage,
    $core.String? commandId,
  }) {
    final result = create();
    if (success != null) result.success = success;
    if (errorMessage != null) result.errorMessage = errorMessage;
    if (commandId != null) result.commandId = commandId;
    return result;
  }

  LockResponse._();

  factory LockResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory LockResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'LockResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.locking'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'success')
    ..aOS(2, _omitFieldNames ? '' : 'errorMessage')
    ..aOS(3, _omitFieldNames ? '' : 'commandId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  LockResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  LockResponse copyWith(void Function(LockResponse) updates) =>
      super.copyWith((message) => updates(message as LockResponse))
          as LockResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static LockResponse create() => LockResponse._();
  @$core.override
  LockResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static LockResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<LockResponse>(create);
  static LockResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.bool get success => $_getBF(0);
  @$pb.TagNumber(1)
  set success($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSuccess() => $_has(0);
  @$pb.TagNumber(1)
  void clearSuccess() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get errorMessage => $_getSZ(1);
  @$pb.TagNumber(2)
  set errorMessage($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasErrorMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearErrorMessage() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get commandId => $_getSZ(2);
  @$pb.TagNumber(3)
  set commandId($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasCommandId() => $_has(2);
  @$pb.TagNumber(3)
  void clearCommandId() => $_clearField(3);
}

class UnlockRequest extends $pb.GeneratedMessage {
  factory UnlockRequest({
    Door? door,
    $core.String? commandId,
    $core.String? authToken,
  }) {
    final result = create();
    if (door != null) result.door = door;
    if (commandId != null) result.commandId = commandId;
    if (authToken != null) result.authToken = authToken;
    return result;
  }

  UnlockRequest._();

  factory UnlockRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory UnlockRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UnlockRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.locking'),
      createEmptyInstance: create)
    ..aE<Door>(1, _omitFieldNames ? '' : 'door', enumValues: Door.values)
    ..aOS(2, _omitFieldNames ? '' : 'commandId')
    ..aOS(3, _omitFieldNames ? '' : 'authToken')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UnlockRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UnlockRequest copyWith(void Function(UnlockRequest) updates) =>
      super.copyWith((message) => updates(message as UnlockRequest))
          as UnlockRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static UnlockRequest create() => UnlockRequest._();
  @$core.override
  UnlockRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static UnlockRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UnlockRequest>(create);
  static UnlockRequest? _defaultInstance;

  @$pb.TagNumber(1)
  Door get door => $_getN(0);
  @$pb.TagNumber(1)
  set door(Door value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasDoor() => $_has(0);
  @$pb.TagNumber(1)
  void clearDoor() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get commandId => $_getSZ(1);
  @$pb.TagNumber(2)
  set commandId($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasCommandId() => $_has(1);
  @$pb.TagNumber(2)
  void clearCommandId() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get authToken => $_getSZ(2);
  @$pb.TagNumber(3)
  set authToken($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasAuthToken() => $_has(2);
  @$pb.TagNumber(3)
  void clearAuthToken() => $_clearField(3);
}

class UnlockResponse extends $pb.GeneratedMessage {
  factory UnlockResponse({
    $core.bool? success,
    $core.String? errorMessage,
    $core.String? commandId,
  }) {
    final result = create();
    if (success != null) result.success = success;
    if (errorMessage != null) result.errorMessage = errorMessage;
    if (commandId != null) result.commandId = commandId;
    return result;
  }

  UnlockResponse._();

  factory UnlockResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory UnlockResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UnlockResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.locking'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'success')
    ..aOS(2, _omitFieldNames ? '' : 'errorMessage')
    ..aOS(3, _omitFieldNames ? '' : 'commandId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UnlockResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UnlockResponse copyWith(void Function(UnlockResponse) updates) =>
      super.copyWith((message) => updates(message as UnlockResponse))
          as UnlockResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static UnlockResponse create() => UnlockResponse._();
  @$core.override
  UnlockResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static UnlockResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UnlockResponse>(create);
  static UnlockResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.bool get success => $_getBF(0);
  @$pb.TagNumber(1)
  set success($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSuccess() => $_has(0);
  @$pb.TagNumber(1)
  void clearSuccess() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get errorMessage => $_getSZ(1);
  @$pb.TagNumber(2)
  set errorMessage($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasErrorMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearErrorMessage() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get commandId => $_getSZ(2);
  @$pb.TagNumber(3)
  set commandId($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasCommandId() => $_has(2);
  @$pb.TagNumber(3)
  void clearCommandId() => $_clearField(3);
}

class GetLockStateRequest extends $pb.GeneratedMessage {
  factory GetLockStateRequest({
    Door? door,
  }) {
    final result = create();
    if (door != null) result.door = door;
    return result;
  }

  GetLockStateRequest._();

  factory GetLockStateRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetLockStateRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetLockStateRequest',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.locking'),
      createEmptyInstance: create)
    ..aE<Door>(1, _omitFieldNames ? '' : 'door', enumValues: Door.values)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetLockStateRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetLockStateRequest copyWith(void Function(GetLockStateRequest) updates) =>
      super.copyWith((message) => updates(message as GetLockStateRequest))
          as GetLockStateRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetLockStateRequest create() => GetLockStateRequest._();
  @$core.override
  GetLockStateRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetLockStateRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetLockStateRequest>(create);
  static GetLockStateRequest? _defaultInstance;

  @$pb.TagNumber(1)
  Door get door => $_getN(0);
  @$pb.TagNumber(1)
  set door(Door value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasDoor() => $_has(0);
  @$pb.TagNumber(1)
  void clearDoor() => $_clearField(1);
}

class GetLockStateResponse extends $pb.GeneratedMessage {
  factory GetLockStateResponse({
    Door? door,
    $core.bool? isLocked,
    $core.bool? isOpen,
  }) {
    final result = create();
    if (door != null) result.door = door;
    if (isLocked != null) result.isLocked = isLocked;
    if (isOpen != null) result.isOpen = isOpen;
    return result;
  }

  GetLockStateResponse._();

  factory GetLockStateResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetLockStateResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetLockStateResponse',
      package: const $pb.PackageName(
          _omitMessageNames ? '' : 'sdv.services.locking'),
      createEmptyInstance: create)
    ..aE<Door>(1, _omitFieldNames ? '' : 'door', enumValues: Door.values)
    ..aOB(2, _omitFieldNames ? '' : 'isLocked')
    ..aOB(3, _omitFieldNames ? '' : 'isOpen')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetLockStateResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetLockStateResponse copyWith(void Function(GetLockStateResponse) updates) =>
      super.copyWith((message) => updates(message as GetLockStateResponse))
          as GetLockStateResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetLockStateResponse create() => GetLockStateResponse._();
  @$core.override
  GetLockStateResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetLockStateResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetLockStateResponse>(create);
  static GetLockStateResponse? _defaultInstance;

  @$pb.TagNumber(1)
  Door get door => $_getN(0);
  @$pb.TagNumber(1)
  set door(Door value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasDoor() => $_has(0);
  @$pb.TagNumber(1)
  void clearDoor() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.bool get isLocked => $_getBF(1);
  @$pb.TagNumber(2)
  set isLocked($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasIsLocked() => $_has(1);
  @$pb.TagNumber(2)
  void clearIsLocked() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.bool get isOpen => $_getBF(2);
  @$pb.TagNumber(3)
  set isOpen($core.bool value) => $_setBool(2, value);
  @$pb.TagNumber(3)
  $core.bool hasIsOpen() => $_has(2);
  @$pb.TagNumber(3)
  void clearIsOpen() => $_clearField(3);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
