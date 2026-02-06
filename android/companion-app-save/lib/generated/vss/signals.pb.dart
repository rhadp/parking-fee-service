// This is a generated file - do not edit.
//
// Generated from vss/signals.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;
import 'package:protobuf/well_known_types/google/protobuf/timestamp.pb.dart'
    as $0;

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

class DoorState extends $pb.GeneratedMessage {
  factory DoorState({
    $core.bool? isLocked,
    $core.bool? isOpen,
    $0.Timestamp? timestamp,
  }) {
    final result = create();
    if (isLocked != null) result.isLocked = isLocked;
    if (isOpen != null) result.isOpen = isOpen;
    if (timestamp != null) result.timestamp = timestamp;
    return result;
  }

  DoorState._();

  factory DoorState.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory DoorState.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'DoorState',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'sdv.vss'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'isLocked')
    ..aOB(2, _omitFieldNames ? '' : 'isOpen')
    ..aOM<$0.Timestamp>(3, _omitFieldNames ? '' : 'timestamp',
        subBuilder: $0.Timestamp.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  DoorState clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  DoorState copyWith(void Function(DoorState) updates) =>
      super.copyWith((message) => updates(message as DoorState)) as DoorState;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static DoorState create() => DoorState._();
  @$core.override
  DoorState createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static DoorState getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<DoorState>(create);
  static DoorState? _defaultInstance;

  @$pb.TagNumber(1)
  $core.bool get isLocked => $_getBF(0);
  @$pb.TagNumber(1)
  set isLocked($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasIsLocked() => $_has(0);
  @$pb.TagNumber(1)
  void clearIsLocked() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.bool get isOpen => $_getBF(1);
  @$pb.TagNumber(2)
  set isOpen($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasIsOpen() => $_has(1);
  @$pb.TagNumber(2)
  void clearIsOpen() => $_clearField(2);

  @$pb.TagNumber(3)
  $0.Timestamp get timestamp => $_getN(2);
  @$pb.TagNumber(3)
  set timestamp($0.Timestamp value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasTimestamp() => $_has(2);
  @$pb.TagNumber(3)
  void clearTimestamp() => $_clearField(3);
  @$pb.TagNumber(3)
  $0.Timestamp ensureTimestamp() => $_ensure(2);
}

class Location extends $pb.GeneratedMessage {
  factory Location({
    $core.double? latitude,
    $core.double? longitude,
    $0.Timestamp? timestamp,
  }) {
    final result = create();
    if (latitude != null) result.latitude = latitude;
    if (longitude != null) result.longitude = longitude;
    if (timestamp != null) result.timestamp = timestamp;
    return result;
  }

  Location._();

  factory Location.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory Location.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Location',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'sdv.vss'),
      createEmptyInstance: create)
    ..aD(1, _omitFieldNames ? '' : 'latitude')
    ..aD(2, _omitFieldNames ? '' : 'longitude')
    ..aOM<$0.Timestamp>(3, _omitFieldNames ? '' : 'timestamp',
        subBuilder: $0.Timestamp.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Location clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Location copyWith(void Function(Location) updates) =>
      super.copyWith((message) => updates(message as Location)) as Location;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static Location create() => Location._();
  @$core.override
  Location createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static Location getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<Location>(create);
  static Location? _defaultInstance;

  @$pb.TagNumber(1)
  $core.double get latitude => $_getN(0);
  @$pb.TagNumber(1)
  set latitude($core.double value) => $_setDouble(0, value);
  @$pb.TagNumber(1)
  $core.bool hasLatitude() => $_has(0);
  @$pb.TagNumber(1)
  void clearLatitude() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.double get longitude => $_getN(1);
  @$pb.TagNumber(2)
  set longitude($core.double value) => $_setDouble(1, value);
  @$pb.TagNumber(2)
  $core.bool hasLongitude() => $_has(1);
  @$pb.TagNumber(2)
  void clearLongitude() => $_clearField(2);

  @$pb.TagNumber(3)
  $0.Timestamp get timestamp => $_getN(2);
  @$pb.TagNumber(3)
  set timestamp($0.Timestamp value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasTimestamp() => $_has(2);
  @$pb.TagNumber(3)
  void clearTimestamp() => $_clearField(3);
  @$pb.TagNumber(3)
  $0.Timestamp ensureTimestamp() => $_ensure(2);
}

class VehicleSpeed extends $pb.GeneratedMessage {
  factory VehicleSpeed({
    $core.double? speedKmh,
    $0.Timestamp? timestamp,
  }) {
    final result = create();
    if (speedKmh != null) result.speedKmh = speedKmh;
    if (timestamp != null) result.timestamp = timestamp;
    return result;
  }

  VehicleSpeed._();

  factory VehicleSpeed.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory VehicleSpeed.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'VehicleSpeed',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'sdv.vss'),
      createEmptyInstance: create)
    ..aD(1, _omitFieldNames ? '' : 'speedKmh', fieldType: $pb.PbFieldType.OF)
    ..aOM<$0.Timestamp>(2, _omitFieldNames ? '' : 'timestamp',
        subBuilder: $0.Timestamp.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VehicleSpeed clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VehicleSpeed copyWith(void Function(VehicleSpeed) updates) =>
      super.copyWith((message) => updates(message as VehicleSpeed))
          as VehicleSpeed;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static VehicleSpeed create() => VehicleSpeed._();
  @$core.override
  VehicleSpeed createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static VehicleSpeed getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<VehicleSpeed>(create);
  static VehicleSpeed? _defaultInstance;

  @$pb.TagNumber(1)
  $core.double get speedKmh => $_getN(0);
  @$pb.TagNumber(1)
  set speedKmh($core.double value) => $_setFloat(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSpeedKmh() => $_has(0);
  @$pb.TagNumber(1)
  void clearSpeedKmh() => $_clearField(1);

  @$pb.TagNumber(2)
  $0.Timestamp get timestamp => $_getN(1);
  @$pb.TagNumber(2)
  set timestamp($0.Timestamp value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasTimestamp() => $_has(1);
  @$pb.TagNumber(2)
  void clearTimestamp() => $_clearField(2);
  @$pb.TagNumber(2)
  $0.Timestamp ensureTimestamp() => $_ensure(1);
}

class ParkingState extends $pb.GeneratedMessage {
  factory ParkingState({
    $core.bool? sessionActive,
    $core.String? sessionId,
    $0.Timestamp? timestamp,
  }) {
    final result = create();
    if (sessionActive != null) result.sessionActive = sessionActive;
    if (sessionId != null) result.sessionId = sessionId;
    if (timestamp != null) result.timestamp = timestamp;
    return result;
  }

  ParkingState._();

  factory ParkingState.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ParkingState.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ParkingState',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'sdv.vss'),
      createEmptyInstance: create)
    ..aOB(1, _omitFieldNames ? '' : 'sessionActive')
    ..aOS(2, _omitFieldNames ? '' : 'sessionId')
    ..aOM<$0.Timestamp>(3, _omitFieldNames ? '' : 'timestamp',
        subBuilder: $0.Timestamp.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ParkingState clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ParkingState copyWith(void Function(ParkingState) updates) =>
      super.copyWith((message) => updates(message as ParkingState))
          as ParkingState;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ParkingState create() => ParkingState._();
  @$core.override
  ParkingState createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ParkingState getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ParkingState>(create);
  static ParkingState? _defaultInstance;

  @$pb.TagNumber(1)
  $core.bool get sessionActive => $_getBF(0);
  @$pb.TagNumber(1)
  set sessionActive($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSessionActive() => $_has(0);
  @$pb.TagNumber(1)
  void clearSessionActive() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get sessionId => $_getSZ(1);
  @$pb.TagNumber(2)
  set sessionId($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasSessionId() => $_has(1);
  @$pb.TagNumber(2)
  void clearSessionId() => $_clearField(2);

  @$pb.TagNumber(3)
  $0.Timestamp get timestamp => $_getN(2);
  @$pb.TagNumber(3)
  set timestamp($0.Timestamp value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasTimestamp() => $_has(2);
  @$pb.TagNumber(3)
  void clearTimestamp() => $_clearField(3);
  @$pb.TagNumber(3)
  $0.Timestamp ensureTimestamp() => $_ensure(2);
}

enum VehicleSignal_Signal { doorState, location, speed, parkingState, notSet }

/// Unified signal container for subscriptions
class VehicleSignal extends $pb.GeneratedMessage {
  factory VehicleSignal({
    DoorState? doorState,
    Location? location,
    VehicleSpeed? speed,
    ParkingState? parkingState,
  }) {
    final result = create();
    if (doorState != null) result.doorState = doorState;
    if (location != null) result.location = location;
    if (speed != null) result.speed = speed;
    if (parkingState != null) result.parkingState = parkingState;
    return result;
  }

  VehicleSignal._();

  factory VehicleSignal.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory VehicleSignal.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static const $core.Map<$core.int, VehicleSignal_Signal>
      _VehicleSignal_SignalByTag = {
    1: VehicleSignal_Signal.doorState,
    2: VehicleSignal_Signal.location,
    3: VehicleSignal_Signal.speed,
    4: VehicleSignal_Signal.parkingState,
    0: VehicleSignal_Signal.notSet
  };
  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'VehicleSignal',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'sdv.vss'),
      createEmptyInstance: create)
    ..oo(0, [1, 2, 3, 4])
    ..aOM<DoorState>(1, _omitFieldNames ? '' : 'doorState',
        subBuilder: DoorState.create)
    ..aOM<Location>(2, _omitFieldNames ? '' : 'location',
        subBuilder: Location.create)
    ..aOM<VehicleSpeed>(3, _omitFieldNames ? '' : 'speed',
        subBuilder: VehicleSpeed.create)
    ..aOM<ParkingState>(4, _omitFieldNames ? '' : 'parkingState',
        subBuilder: ParkingState.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VehicleSignal clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VehicleSignal copyWith(void Function(VehicleSignal) updates) =>
      super.copyWith((message) => updates(message as VehicleSignal))
          as VehicleSignal;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static VehicleSignal create() => VehicleSignal._();
  @$core.override
  VehicleSignal createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static VehicleSignal getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<VehicleSignal>(create);
  static VehicleSignal? _defaultInstance;

  @$pb.TagNumber(1)
  @$pb.TagNumber(2)
  @$pb.TagNumber(3)
  @$pb.TagNumber(4)
  VehicleSignal_Signal whichSignal() =>
      _VehicleSignal_SignalByTag[$_whichOneof(0)]!;
  @$pb.TagNumber(1)
  @$pb.TagNumber(2)
  @$pb.TagNumber(3)
  @$pb.TagNumber(4)
  void clearSignal() => $_clearField($_whichOneof(0));

  @$pb.TagNumber(1)
  DoorState get doorState => $_getN(0);
  @$pb.TagNumber(1)
  set doorState(DoorState value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasDoorState() => $_has(0);
  @$pb.TagNumber(1)
  void clearDoorState() => $_clearField(1);
  @$pb.TagNumber(1)
  DoorState ensureDoorState() => $_ensure(0);

  @$pb.TagNumber(2)
  Location get location => $_getN(1);
  @$pb.TagNumber(2)
  set location(Location value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasLocation() => $_has(1);
  @$pb.TagNumber(2)
  void clearLocation() => $_clearField(2);
  @$pb.TagNumber(2)
  Location ensureLocation() => $_ensure(1);

  @$pb.TagNumber(3)
  VehicleSpeed get speed => $_getN(2);
  @$pb.TagNumber(3)
  set speed(VehicleSpeed value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasSpeed() => $_has(2);
  @$pb.TagNumber(3)
  void clearSpeed() => $_clearField(3);
  @$pb.TagNumber(3)
  VehicleSpeed ensureSpeed() => $_ensure(2);

  @$pb.TagNumber(4)
  ParkingState get parkingState => $_getN(3);
  @$pb.TagNumber(4)
  set parkingState(ParkingState value) => $_setField(4, value);
  @$pb.TagNumber(4)
  $core.bool hasParkingState() => $_has(3);
  @$pb.TagNumber(4)
  void clearParkingState() => $_clearField(4);
  @$pb.TagNumber(4)
  ParkingState ensureParkingState() => $_ensure(3);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
