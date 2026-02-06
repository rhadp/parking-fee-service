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

@$core.Deprecated('Use startSessionRequestDescriptor instead')
const StartSessionRequest$json = {
  '1': 'StartSessionRequest',
  '2': [
    {'1': 'vehicle_id', '3': 1, '4': 1, '5': 9, '10': 'vehicleId'},
    {'1': 'zone_id', '3': 2, '4': 1, '5': 9, '10': 'zoneId'},
    {'1': 'latitude', '3': 3, '4': 1, '5': 1, '10': 'latitude'},
    {'1': 'longitude', '3': 4, '4': 1, '5': 1, '10': 'longitude'},
  ],
};

/// Descriptor for `StartSessionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List startSessionRequestDescriptor = $convert.base64Decode(
    'ChNTdGFydFNlc3Npb25SZXF1ZXN0Eh0KCnZlaGljbGVfaWQYASABKAlSCXZlaGljbGVJZBIXCg'
    'd6b25lX2lkGAIgASgJUgZ6b25lSWQSGgoIbGF0aXR1ZGUYAyABKAFSCGxhdGl0dWRlEhwKCWxv'
    'bmdpdHVkZRgEIAEoAVIJbG9uZ2l0dWRl');

@$core.Deprecated('Use startSessionResponseDescriptor instead')
const StartSessionResponse$json = {
  '1': 'StartSessionResponse',
  '2': [
    {'1': 'session_id', '3': 1, '4': 1, '5': 9, '10': 'sessionId'},
    {'1': 'success', '3': 2, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 3, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'operator_name', '3': 4, '4': 1, '5': 9, '10': 'operatorName'},
    {'1': 'hourly_rate', '3': 5, '4': 1, '5': 1, '10': 'hourlyRate'},
    {'1': 'currency', '3': 6, '4': 1, '5': 9, '10': 'currency'},
  ],
};

/// Descriptor for `StartSessionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List startSessionResponseDescriptor = $convert.base64Decode(
    'ChRTdGFydFNlc3Npb25SZXNwb25zZRIdCgpzZXNzaW9uX2lkGAEgASgJUglzZXNzaW9uSWQSGA'
    'oHc3VjY2VzcxgCIAEoCFIHc3VjY2VzcxIjCg1lcnJvcl9tZXNzYWdlGAMgASgJUgxlcnJvck1l'
    'c3NhZ2USIwoNb3BlcmF0b3JfbmFtZRgEIAEoCVIMb3BlcmF0b3JOYW1lEh8KC2hvdXJseV9yYX'
    'RlGAUgASgBUgpob3VybHlSYXRlEhoKCGN1cnJlbmN5GAYgASgJUghjdXJyZW5jeQ==');

@$core.Deprecated('Use stopSessionRequestDescriptor instead')
const StopSessionRequest$json = {
  '1': 'StopSessionRequest',
  '2': [
    {'1': 'session_id', '3': 1, '4': 1, '5': 9, '10': 'sessionId'},
  ],
};

/// Descriptor for `StopSessionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List stopSessionRequestDescriptor =
    $convert.base64Decode(
        'ChJTdG9wU2Vzc2lvblJlcXVlc3QSHQoKc2Vzc2lvbl9pZBgBIAEoCVIJc2Vzc2lvbklk');

@$core.Deprecated('Use stopSessionResponseDescriptor instead')
const StopSessionResponse$json = {
  '1': 'StopSessionResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'total_amount', '3': 3, '4': 1, '5': 1, '10': 'totalAmount'},
    {'1': 'currency', '3': 4, '4': 1, '5': 9, '10': 'currency'},
    {'1': 'duration_seconds', '3': 5, '4': 1, '5': 3, '10': 'durationSeconds'},
  ],
};

/// Descriptor for `StopSessionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List stopSessionResponseDescriptor = $convert.base64Decode(
    'ChNTdG9wU2Vzc2lvblJlc3BvbnNlEhgKB3N1Y2Nlc3MYASABKAhSB3N1Y2Nlc3MSIwoNZXJyb3'
    'JfbWVzc2FnZRgCIAEoCVIMZXJyb3JNZXNzYWdlEiEKDHRvdGFsX2Ftb3VudBgDIAEoAVILdG90'
    'YWxBbW91bnQSGgoIY3VycmVuY3kYBCABKAlSCGN1cnJlbmN5EikKEGR1cmF0aW9uX3NlY29uZH'
    'MYBSABKANSD2R1cmF0aW9uU2Vjb25kcw==');

@$core.Deprecated('Use getSessionStatusRequestDescriptor instead')
const GetSessionStatusRequest$json = {
  '1': 'GetSessionStatusRequest',
  '2': [
    {'1': 'session_id', '3': 1, '4': 1, '5': 9, '10': 'sessionId'},
  ],
};

/// Descriptor for `GetSessionStatusRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSessionStatusRequestDescriptor =
    $convert.base64Decode(
        'ChdHZXRTZXNzaW9uU3RhdHVzUmVxdWVzdBIdCgpzZXNzaW9uX2lkGAEgASgJUglzZXNzaW9uSW'
        'Q=');

@$core.Deprecated('Use getSessionStatusResponseDescriptor instead')
const GetSessionStatusResponse$json = {
  '1': 'GetSessionStatusResponse',
  '2': [
    {'1': 'session_id', '3': 1, '4': 1, '5': 9, '10': 'sessionId'},
    {'1': 'active', '3': 2, '4': 1, '5': 8, '10': 'active'},
    {
      '1': 'start_time',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.google.protobuf.Timestamp',
      '10': 'startTime'
    },
    {'1': 'current_amount', '3': 4, '4': 1, '5': 1, '10': 'currentAmount'},
    {'1': 'currency', '3': 5, '4': 1, '5': 9, '10': 'currency'},
  ],
};

/// Descriptor for `GetSessionStatusResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSessionStatusResponseDescriptor = $convert.base64Decode(
    'ChhHZXRTZXNzaW9uU3RhdHVzUmVzcG9uc2USHQoKc2Vzc2lvbl9pZBgBIAEoCVIJc2Vzc2lvbk'
    'lkEhYKBmFjdGl2ZRgCIAEoCFIGYWN0aXZlEjkKCnN0YXJ0X3RpbWUYAyABKAsyGi5nb29nbGUu'
    'cHJvdG9idWYuVGltZXN0YW1wUglzdGFydFRpbWUSJQoOY3VycmVudF9hbW91bnQYBCABKAFSDW'
    'N1cnJlbnRBbW91bnQSGgoIY3VycmVuY3kYBSABKAlSCGN1cnJlbmN5');
