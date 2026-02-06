// This is a generated file - do not edit.
//
// Generated from vss/signals.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports
// ignore_for_file: unused_import

import 'dart:convert' as $convert;
import 'dart:core' as $core;
import 'dart:typed_data' as $typed_data;

@$core.Deprecated('Use doorStateDescriptor instead')
const DoorState$json = {
  '1': 'DoorState',
  '2': [
    {'1': 'is_locked', '3': 1, '4': 1, '5': 8, '10': 'isLocked'},
    {'1': 'is_open', '3': 2, '4': 1, '5': 8, '10': 'isOpen'},
    {
      '1': 'timestamp',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.google.protobuf.Timestamp',
      '10': 'timestamp'
    },
  ],
};

/// Descriptor for `DoorState`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List doorStateDescriptor = $convert.base64Decode(
    'CglEb29yU3RhdGUSGwoJaXNfbG9ja2VkGAEgASgIUghpc0xvY2tlZBIXCgdpc19vcGVuGAIgAS'
    'gIUgZpc09wZW4SOAoJdGltZXN0YW1wGAMgASgLMhouZ29vZ2xlLnByb3RvYnVmLlRpbWVzdGFt'
    'cFIJdGltZXN0YW1w');

@$core.Deprecated('Use locationDescriptor instead')
const Location$json = {
  '1': 'Location',
  '2': [
    {'1': 'latitude', '3': 1, '4': 1, '5': 1, '10': 'latitude'},
    {'1': 'longitude', '3': 2, '4': 1, '5': 1, '10': 'longitude'},
    {
      '1': 'timestamp',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.google.protobuf.Timestamp',
      '10': 'timestamp'
    },
  ],
};

/// Descriptor for `Location`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List locationDescriptor = $convert.base64Decode(
    'CghMb2NhdGlvbhIaCghsYXRpdHVkZRgBIAEoAVIIbGF0aXR1ZGUSHAoJbG9uZ2l0dWRlGAIgAS'
    'gBUglsb25naXR1ZGUSOAoJdGltZXN0YW1wGAMgASgLMhouZ29vZ2xlLnByb3RvYnVmLlRpbWVz'
    'dGFtcFIJdGltZXN0YW1w');

@$core.Deprecated('Use vehicleSpeedDescriptor instead')
const VehicleSpeed$json = {
  '1': 'VehicleSpeed',
  '2': [
    {'1': 'speed_kmh', '3': 1, '4': 1, '5': 2, '10': 'speedKmh'},
    {
      '1': 'timestamp',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.google.protobuf.Timestamp',
      '10': 'timestamp'
    },
  ],
};

/// Descriptor for `VehicleSpeed`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List vehicleSpeedDescriptor = $convert.base64Decode(
    'CgxWZWhpY2xlU3BlZWQSGwoJc3BlZWRfa21oGAEgASgCUghzcGVlZEttaBI4Cgl0aW1lc3RhbX'
    'AYAiABKAsyGi5nb29nbGUucHJvdG9idWYuVGltZXN0YW1wUgl0aW1lc3RhbXA=');

@$core.Deprecated('Use parkingStateDescriptor instead')
const ParkingState$json = {
  '1': 'ParkingState',
  '2': [
    {'1': 'session_active', '3': 1, '4': 1, '5': 8, '10': 'sessionActive'},
    {'1': 'session_id', '3': 2, '4': 1, '5': 9, '10': 'sessionId'},
    {
      '1': 'timestamp',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.google.protobuf.Timestamp',
      '10': 'timestamp'
    },
  ],
};

/// Descriptor for `ParkingState`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List parkingStateDescriptor = $convert.base64Decode(
    'CgxQYXJraW5nU3RhdGUSJQoOc2Vzc2lvbl9hY3RpdmUYASABKAhSDXNlc3Npb25BY3RpdmUSHQ'
    'oKc2Vzc2lvbl9pZBgCIAEoCVIJc2Vzc2lvbklkEjgKCXRpbWVzdGFtcBgDIAEoCzIaLmdvb2ds'
    'ZS5wcm90b2J1Zi5UaW1lc3RhbXBSCXRpbWVzdGFtcA==');

@$core.Deprecated('Use vehicleSignalDescriptor instead')
const VehicleSignal$json = {
  '1': 'VehicleSignal',
  '2': [
    {
      '1': 'door_state',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.sdv.vss.DoorState',
      '9': 0,
      '10': 'doorState'
    },
    {
      '1': 'location',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.sdv.vss.Location',
      '9': 0,
      '10': 'location'
    },
    {
      '1': 'speed',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.sdv.vss.VehicleSpeed',
      '9': 0,
      '10': 'speed'
    },
    {
      '1': 'parking_state',
      '3': 4,
      '4': 1,
      '5': 11,
      '6': '.sdv.vss.ParkingState',
      '9': 0,
      '10': 'parkingState'
    },
  ],
  '8': [
    {'1': 'signal'},
  ],
};

/// Descriptor for `VehicleSignal`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List vehicleSignalDescriptor = $convert.base64Decode(
    'Cg1WZWhpY2xlU2lnbmFsEjMKCmRvb3Jfc3RhdGUYASABKAsyEi5zZHYudnNzLkRvb3JTdGF0ZU'
    'gAUglkb29yU3RhdGUSLwoIbG9jYXRpb24YAiABKAsyES5zZHYudnNzLkxvY2F0aW9uSABSCGxv'
    'Y2F0aW9uEi0KBXNwZWVkGAMgASgLMhUuc2R2LnZzcy5WZWhpY2xlU3BlZWRIAFIFc3BlZWQSPA'
    'oNcGFya2luZ19zdGF0ZRgEIAEoCzIVLnNkdi52c3MuUGFya2luZ1N0YXRlSABSDHBhcmtpbmdT'
    'dGF0ZUIICgZzaWduYWw=');
