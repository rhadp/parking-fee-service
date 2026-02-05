// This is a generated file - do not edit.
//
// Generated from services/update_service.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

import 'update_service.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'update_service.pbenum.dart';

/// AdapterInfo contains metadata about an installed adapter
class AdapterInfo extends $pb.GeneratedMessage {
  factory AdapterInfo({
    $core.String? adapterId,
    $core.String? imageRef,
    $core.String? version,
    AdapterState? state,
    $core.String? errorMessage,
  }) {
    final result = create();
    if (adapterId != null) result.adapterId = adapterId;
    if (imageRef != null) result.imageRef = imageRef;
    if (version != null) result.version = version;
    if (state != null) result.state = state;
    if (errorMessage != null) result.errorMessage = errorMessage;
    return result;
  }

  AdapterInfo._();

  factory AdapterInfo.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory AdapterInfo.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'AdapterInfo',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'adapterId')
    ..aOS(2, _omitFieldNames ? '' : 'imageRef')
    ..aOS(3, _omitFieldNames ? '' : 'version')
    ..aE<AdapterState>(4, _omitFieldNames ? '' : 'state',
        enumValues: AdapterState.values)
    ..aOS(5, _omitFieldNames ? '' : 'errorMessage')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  AdapterInfo clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  AdapterInfo copyWith(void Function(AdapterInfo) updates) =>
      super.copyWith((message) => updates(message as AdapterInfo))
          as AdapterInfo;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static AdapterInfo create() => AdapterInfo._();
  @$core.override
  AdapterInfo createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static AdapterInfo getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<AdapterInfo>(create);
  static AdapterInfo? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get adapterId => $_getSZ(0);
  @$pb.TagNumber(1)
  set adapterId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasAdapterId() => $_has(0);
  @$pb.TagNumber(1)
  void clearAdapterId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get imageRef => $_getSZ(1);
  @$pb.TagNumber(2)
  set imageRef($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasImageRef() => $_has(1);
  @$pb.TagNumber(2)
  void clearImageRef() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get version => $_getSZ(2);
  @$pb.TagNumber(3)
  set version($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasVersion() => $_has(2);
  @$pb.TagNumber(3)
  void clearVersion() => $_clearField(3);

  @$pb.TagNumber(4)
  AdapterState get state => $_getN(3);
  @$pb.TagNumber(4)
  set state(AdapterState value) => $_setField(4, value);
  @$pb.TagNumber(4)
  $core.bool hasState() => $_has(3);
  @$pb.TagNumber(4)
  void clearState() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.String get errorMessage => $_getSZ(4);
  @$pb.TagNumber(5)
  set errorMessage($core.String value) => $_setString(4, value);
  @$pb.TagNumber(5)
  $core.bool hasErrorMessage() => $_has(4);
  @$pb.TagNumber(5)
  void clearErrorMessage() => $_clearField(5);
}

/// InstallAdapterRequest contains the information needed to install an adapter
class InstallAdapterRequest extends $pb.GeneratedMessage {
  factory InstallAdapterRequest({
    $core.String? imageRef,
    $core.String? checksum,
  }) {
    final result = create();
    if (imageRef != null) result.imageRef = imageRef;
    if (checksum != null) result.checksum = checksum;
    return result;
  }

  InstallAdapterRequest._();

  factory InstallAdapterRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory InstallAdapterRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'InstallAdapterRequest',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'imageRef')
    ..aOS(2, _omitFieldNames ? '' : 'checksum')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  InstallAdapterRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  InstallAdapterRequest copyWith(
          void Function(InstallAdapterRequest) updates) =>
      super.copyWith((message) => updates(message as InstallAdapterRequest))
          as InstallAdapterRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static InstallAdapterRequest create() => InstallAdapterRequest._();
  @$core.override
  InstallAdapterRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static InstallAdapterRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<InstallAdapterRequest>(create);
  static InstallAdapterRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get imageRef => $_getSZ(0);
  @$pb.TagNumber(1)
  set imageRef($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasImageRef() => $_has(0);
  @$pb.TagNumber(1)
  void clearImageRef() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get checksum => $_getSZ(1);
  @$pb.TagNumber(2)
  set checksum($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasChecksum() => $_has(1);
  @$pb.TagNumber(2)
  void clearChecksum() => $_clearField(2);
}

/// InstallAdapterResponse contains the result of an adapter installation request
class InstallAdapterResponse extends $pb.GeneratedMessage {
  factory InstallAdapterResponse({
    $core.String? jobId,
    $core.String? adapterId,
    AdapterState? state,
  }) {
    final result = create();
    if (jobId != null) result.jobId = jobId;
    if (adapterId != null) result.adapterId = adapterId;
    if (state != null) result.state = state;
    return result;
  }

  InstallAdapterResponse._();

  factory InstallAdapterResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory InstallAdapterResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'InstallAdapterResponse',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'jobId')
    ..aOS(2, _omitFieldNames ? '' : 'adapterId')
    ..aE<AdapterState>(3, _omitFieldNames ? '' : 'state',
        enumValues: AdapterState.values)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  InstallAdapterResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  InstallAdapterResponse copyWith(
          void Function(InstallAdapterResponse) updates) =>
      super.copyWith((message) => updates(message as InstallAdapterResponse))
          as InstallAdapterResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static InstallAdapterResponse create() => InstallAdapterResponse._();
  @$core.override
  InstallAdapterResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static InstallAdapterResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<InstallAdapterResponse>(create);
  static InstallAdapterResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get jobId => $_getSZ(0);
  @$pb.TagNumber(1)
  set jobId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasJobId() => $_has(0);
  @$pb.TagNumber(1)
  void clearJobId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get adapterId => $_getSZ(1);
  @$pb.TagNumber(2)
  set adapterId($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasAdapterId() => $_has(1);
  @$pb.TagNumber(2)
  void clearAdapterId() => $_clearField(2);

  @$pb.TagNumber(3)
  AdapterState get state => $_getN(2);
  @$pb.TagNumber(3)
  set state(AdapterState value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasState() => $_has(2);
  @$pb.TagNumber(3)
  void clearState() => $_clearField(3);
}

/// UninstallAdapterRequest identifies the adapter to uninstall
class UninstallAdapterRequest extends $pb.GeneratedMessage {
  factory UninstallAdapterRequest({
    $core.String? adapterId,
  }) {
    final result = create();
    if (adapterId != null) result.adapterId = adapterId;
    return result;
  }

  UninstallAdapterRequest._();

  factory UninstallAdapterRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory UninstallAdapterRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UninstallAdapterRequest',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'adapterId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UninstallAdapterRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UninstallAdapterRequest copyWith(
          void Function(UninstallAdapterRequest) updates) =>
      super.copyWith((message) => updates(message as UninstallAdapterRequest))
          as UninstallAdapterRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static UninstallAdapterRequest create() => UninstallAdapterRequest._();
  @$core.override
  UninstallAdapterRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static UninstallAdapterRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UninstallAdapterRequest>(create);
  static UninstallAdapterRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get adapterId => $_getSZ(0);
  @$pb.TagNumber(1)
  set adapterId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasAdapterId() => $_has(0);
  @$pb.TagNumber(1)
  void clearAdapterId() => $_clearField(1);
}

/// UninstallAdapterResponse contains the result of an adapter uninstallation request
class UninstallAdapterResponse extends $pb.GeneratedMessage {
  factory UninstallAdapterResponse({
    $core.bool? success,
    $core.String? errorMessage,
  }) {
    final result = create();
    if (success != null) result.success = success;
    if (errorMessage != null) result.errorMessage = errorMessage;
    return result;
  }

  UninstallAdapterResponse._();

  factory UninstallAdapterResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory UninstallAdapterResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UninstallAdapterResponse',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'success')
    ..aOS(2, _omitFieldNames ? '' : 'errorMessage')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UninstallAdapterResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UninstallAdapterResponse copyWith(
          void Function(UninstallAdapterResponse) updates) =>
      super.copyWith((message) => updates(message as UninstallAdapterResponse))
          as UninstallAdapterResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static UninstallAdapterResponse create() => UninstallAdapterResponse._();
  @$core.override
  UninstallAdapterResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static UninstallAdapterResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UninstallAdapterResponse>(create);
  static UninstallAdapterResponse? _defaultInstance;

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
}

/// ListAdaptersRequest is an empty request to list all installed adapters
class ListAdaptersRequest extends $pb.GeneratedMessage {
  factory ListAdaptersRequest() => create();

  ListAdaptersRequest._();

  factory ListAdaptersRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ListAdaptersRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListAdaptersRequest',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAdaptersRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAdaptersRequest copyWith(void Function(ListAdaptersRequest) updates) =>
      super.copyWith((message) => updates(message as ListAdaptersRequest))
          as ListAdaptersRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ListAdaptersRequest create() => ListAdaptersRequest._();
  @$core.override
  ListAdaptersRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ListAdaptersRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListAdaptersRequest>(create);
  static ListAdaptersRequest? _defaultInstance;
}

/// ListAdaptersResponse contains the list of installed adapters
class ListAdaptersResponse extends $pb.GeneratedMessage {
  factory ListAdaptersResponse({
    $core.Iterable<AdapterInfo>? adapters,
  }) {
    final result = create();
    if (adapters != null) result.adapters.addAll(adapters);
    return result;
  }

  ListAdaptersResponse._();

  factory ListAdaptersResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ListAdaptersResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListAdaptersResponse',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..pPM<AdapterInfo>(1, _omitFieldNames ? '' : 'adapters',
        subBuilder: AdapterInfo.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAdaptersResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAdaptersResponse copyWith(void Function(ListAdaptersResponse) updates) =>
      super.copyWith((message) => updates(message as ListAdaptersResponse))
          as ListAdaptersResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ListAdaptersResponse create() => ListAdaptersResponse._();
  @$core.override
  ListAdaptersResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ListAdaptersResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListAdaptersResponse>(create);
  static ListAdaptersResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<AdapterInfo> get adapters => $_getList(0);
}

/// WatchAdapterStatesRequest specifies which adapters to watch for state changes
class WatchAdapterStatesRequest extends $pb.GeneratedMessage {
  factory WatchAdapterStatesRequest({
    $core.Iterable<$core.String>? adapterIds,
  }) {
    final result = create();
    if (adapterIds != null) result.adapterIds.addAll(adapterIds);
    return result;
  }

  WatchAdapterStatesRequest._();

  factory WatchAdapterStatesRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory WatchAdapterStatesRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'WatchAdapterStatesRequest',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..pPS(1, _omitFieldNames ? '' : 'adapterIds')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  WatchAdapterStatesRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  WatchAdapterStatesRequest copyWith(
          void Function(WatchAdapterStatesRequest) updates) =>
      super.copyWith((message) => updates(message as WatchAdapterStatesRequest))
          as WatchAdapterStatesRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static WatchAdapterStatesRequest create() => WatchAdapterStatesRequest._();
  @$core.override
  WatchAdapterStatesRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static WatchAdapterStatesRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<WatchAdapterStatesRequest>(create);
  static WatchAdapterStatesRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get adapterIds => $_getList(0);
}

/// AdapterStateEvent represents a state change event for an adapter
class AdapterStateEvent extends $pb.GeneratedMessage {
  factory AdapterStateEvent({
    $core.String? adapterId,
    AdapterState? oldState,
    AdapterState? newState,
    $core.String? errorMessage,
  }) {
    final result = create();
    if (adapterId != null) result.adapterId = adapterId;
    if (oldState != null) result.oldState = oldState;
    if (newState != null) result.newState = newState;
    if (errorMessage != null) result.errorMessage = errorMessage;
    return result;
  }

  AdapterStateEvent._();

  factory AdapterStateEvent.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory AdapterStateEvent.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'AdapterStateEvent',
      package:
          const $pb.PackageName(_omitMessageNames ? '' : 'sdv.services.update'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'adapterId')
    ..aE<AdapterState>(2, _omitFieldNames ? '' : 'oldState',
        enumValues: AdapterState.values)
    ..aE<AdapterState>(3, _omitFieldNames ? '' : 'newState',
        enumValues: AdapterState.values)
    ..aOS(4, _omitFieldNames ? '' : 'errorMessage')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  AdapterStateEvent clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  AdapterStateEvent copyWith(void Function(AdapterStateEvent) updates) =>
      super.copyWith((message) => updates(message as AdapterStateEvent))
          as AdapterStateEvent;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static AdapterStateEvent create() => AdapterStateEvent._();
  @$core.override
  AdapterStateEvent createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static AdapterStateEvent getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<AdapterStateEvent>(create);
  static AdapterStateEvent? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get adapterId => $_getSZ(0);
  @$pb.TagNumber(1)
  set adapterId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasAdapterId() => $_has(0);
  @$pb.TagNumber(1)
  void clearAdapterId() => $_clearField(1);

  @$pb.TagNumber(2)
  AdapterState get oldState => $_getN(1);
  @$pb.TagNumber(2)
  set oldState(AdapterState value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasOldState() => $_has(1);
  @$pb.TagNumber(2)
  void clearOldState() => $_clearField(2);

  @$pb.TagNumber(3)
  AdapterState get newState => $_getN(2);
  @$pb.TagNumber(3)
  set newState(AdapterState value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasNewState() => $_has(2);
  @$pb.TagNumber(3)
  void clearNewState() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get errorMessage => $_getSZ(3);
  @$pb.TagNumber(4)
  set errorMessage($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasErrorMessage() => $_has(3);
  @$pb.TagNumber(4)
  void clearErrorMessage() => $_clearField(4);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
