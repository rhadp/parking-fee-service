// This is a generated file - do not edit.
//
// Generated from services/parking_adaptor.proto.

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

@$core.Deprecated('Use sessionStateDescriptor instead')
const SessionState$json = {
  '1': 'SessionState',
  '2': [
    {'1': 'SESSION_STATE_NONE', '2': 0},
    {'1': 'SESSION_STATE_STARTING', '2': 1},
    {'1': 'SESSION_STATE_ACTIVE', '2': 2},
    {'1': 'SESSION_STATE_STOPPING', '2': 3},
    {'1': 'SESSION_STATE_STOPPED', '2': 4},
    {'1': 'SESSION_STATE_ERROR', '2': 5},
  ],
};

/// Descriptor for `SessionState`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List sessionStateDescriptor = $convert.base64Decode(
    'CgxTZXNzaW9uU3RhdGUSFgoSU0VTU0lPTl9TVEFURV9OT05FEAASGgoWU0VTU0lPTl9TVEFURV'
    '9TVEFSVElORxABEhgKFFNFU1NJT05fU1RBVEVfQUNUSVZFEAISGgoWU0VTU0lPTl9TVEFURV9T'
    'VE9QUElORxADEhkKFVNFU1NJT05fU1RBVEVfU1RPUFBFRBAEEhcKE1NFU1NJT05fU1RBVEVfRV'
    'JST1IQBQ==');

@$core.Deprecated('Use startSessionRequestDescriptor instead')
const StartSessionRequest$json = {
  '1': 'StartSessionRequest',
  '2': [
    {'1': 'zone_id', '3': 1, '4': 1, '5': 9, '10': 'zoneId'},
  ],
};

/// Descriptor for `StartSessionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List startSessionRequestDescriptor =
    $convert.base64Decode(
        'ChNTdGFydFNlc3Npb25SZXF1ZXN0EhcKB3pvbmVfaWQYASABKAlSBnpvbmVJZA==');

@$core.Deprecated('Use startSessionResponseDescriptor instead')
const StartSessionResponse$json = {
  '1': 'StartSessionResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'session_id', '3': 3, '4': 1, '5': 9, '10': 'sessionId'},
    {
      '1': 'state',
      '3': 4,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.parking.SessionState',
      '10': 'state'
    },
  ],
};

/// Descriptor for `StartSessionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List startSessionResponseDescriptor = $convert.base64Decode(
    'ChRTdGFydFNlc3Npb25SZXNwb25zZRIYCgdzdWNjZXNzGAEgASgIUgdzdWNjZXNzEiMKDWVycm'
    '9yX21lc3NhZ2UYAiABKAlSDGVycm9yTWVzc2FnZRIdCgpzZXNzaW9uX2lkGAMgASgJUglzZXNz'
    'aW9uSWQSOAoFc3RhdGUYBCABKA4yIi5zZHYuc2VydmljZXMucGFya2luZy5TZXNzaW9uU3RhdG'
    'VSBXN0YXRl');

@$core.Deprecated('Use stopSessionRequestDescriptor instead')
const StopSessionRequest$json = {
  '1': 'StopSessionRequest',
};

/// Descriptor for `StopSessionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List stopSessionRequestDescriptor =
    $convert.base64Decode('ChJTdG9wU2Vzc2lvblJlcXVlc3Q=');

@$core.Deprecated('Use stopSessionResponseDescriptor instead')
const StopSessionResponse$json = {
  '1': 'StopSessionResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'session_id', '3': 3, '4': 1, '5': 9, '10': 'sessionId'},
    {
      '1': 'state',
      '3': 4,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.parking.SessionState',
      '10': 'state'
    },
    {'1': 'final_cost', '3': 5, '4': 1, '5': 1, '10': 'finalCost'},
    {'1': 'duration_seconds', '3': 6, '4': 1, '5': 3, '10': 'durationSeconds'},
  ],
};

/// Descriptor for `StopSessionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List stopSessionResponseDescriptor = $convert.base64Decode(
    'ChNTdG9wU2Vzc2lvblJlc3BvbnNlEhgKB3N1Y2Nlc3MYASABKAhSB3N1Y2Nlc3MSIwoNZXJyb3'
    'JfbWVzc2FnZRgCIAEoCVIMZXJyb3JNZXNzYWdlEh0KCnNlc3Npb25faWQYAyABKAlSCXNlc3Np'
    'b25JZBI4CgVzdGF0ZRgEIAEoDjIiLnNkdi5zZXJ2aWNlcy5wYXJraW5nLlNlc3Npb25TdGF0ZV'
    'IFc3RhdGUSHQoKZmluYWxfY29zdBgFIAEoAVIJZmluYWxDb3N0EikKEGR1cmF0aW9uX3NlY29u'
    'ZHMYBiABKANSD2R1cmF0aW9uU2Vjb25kcw==');

@$core.Deprecated('Use getSessionStatusRequestDescriptor instead')
const GetSessionStatusRequest$json = {
  '1': 'GetSessionStatusRequest',
};

/// Descriptor for `GetSessionStatusRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSessionStatusRequestDescriptor =
    $convert.base64Decode('ChdHZXRTZXNzaW9uU3RhdHVzUmVxdWVzdA==');

@$core.Deprecated('Use getSessionStatusResponseDescriptor instead')
const GetSessionStatusResponse$json = {
  '1': 'GetSessionStatusResponse',
  '2': [
    {
      '1': 'has_active_session',
      '3': 1,
      '4': 1,
      '5': 8,
      '10': 'hasActiveSession'
    },
    {'1': 'session_id', '3': 2, '4': 1, '5': 9, '10': 'sessionId'},
    {
      '1': 'state',
      '3': 3,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.parking.SessionState',
      '10': 'state'
    },
    {'1': 'start_time_unix', '3': 4, '4': 1, '5': 3, '10': 'startTimeUnix'},
    {'1': 'duration_seconds', '3': 5, '4': 1, '5': 3, '10': 'durationSeconds'},
    {'1': 'current_cost', '3': 6, '4': 1, '5': 1, '10': 'currentCost'},
    {'1': 'zone_id', '3': 7, '4': 1, '5': 9, '10': 'zoneId'},
    {'1': 'error_message', '3': 8, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'latitude', '3': 9, '4': 1, '5': 1, '10': 'latitude'},
    {'1': 'longitude', '3': 10, '4': 1, '5': 1, '10': 'longitude'},
  ],
};

/// Descriptor for `GetSessionStatusResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSessionStatusResponseDescriptor = $convert.base64Decode(
    'ChhHZXRTZXNzaW9uU3RhdHVzUmVzcG9uc2USLAoSaGFzX2FjdGl2ZV9zZXNzaW9uGAEgASgIUh'
    'BoYXNBY3RpdmVTZXNzaW9uEh0KCnNlc3Npb25faWQYAiABKAlSCXNlc3Npb25JZBI4CgVzdGF0'
    'ZRgDIAEoDjIiLnNkdi5zZXJ2aWNlcy5wYXJraW5nLlNlc3Npb25TdGF0ZVIFc3RhdGUSJgoPc3'
    'RhcnRfdGltZV91bml4GAQgASgDUg1zdGFydFRpbWVVbml4EikKEGR1cmF0aW9uX3NlY29uZHMY'
    'BSABKANSD2R1cmF0aW9uU2Vjb25kcxIhCgxjdXJyZW50X2Nvc3QYBiABKAFSC2N1cnJlbnRDb3'
    'N0EhcKB3pvbmVfaWQYByABKAlSBnpvbmVJZBIjCg1lcnJvcl9tZXNzYWdlGAggASgJUgxlcnJv'
    'ck1lc3NhZ2USGgoIbGF0aXR1ZGUYCSABKAFSCGxhdGl0dWRlEhwKCWxvbmdpdHVkZRgKIAEoAV'
    'IJbG9uZ2l0dWRl');
