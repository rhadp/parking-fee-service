// This is a generated file - do not edit.
//
// Generated from common/error.proto.

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

@$core.Deprecated('Use errorCodeDescriptor instead')
const ErrorCode$json = {
  '1': 'ErrorCode',
  '2': [
    {'1': 'ERROR_CODE_UNSPECIFIED', '2': 0},
    {'1': 'ERROR_CODE_INVALID_REQUEST', '2': 1},
    {'1': 'ERROR_CODE_SERVICE_NOT_READY', '2': 2},
    {'1': 'ERROR_CODE_REGISTRY_UNAVAILABLE', '2': 3},
    {'1': 'ERROR_CODE_CHECKSUM_MISMATCH', '2': 4},
    {'1': 'ERROR_CODE_TLS_ERROR', '2': 5},
    {'1': 'ERROR_CODE_CONNECTION_FAILED', '2': 6},
    {'1': 'ERROR_CODE_AUTH_FAILED', '2': 7},
    {'1': 'ERROR_CODE_PERMISSION_DENIED', '2': 8},
    {'1': 'ERROR_CODE_NOT_FOUND', '2': 9},
    {'1': 'ERROR_CODE_TIMEOUT', '2': 10},
    {'1': 'ERROR_CODE_INTERNAL_ERROR', '2': 11},
    {'1': 'ERROR_CODE_ADAPTER_NOT_FOUND', '2': 12},
    {'1': 'ERROR_CODE_ADAPTER_ALREADY_INSTALLED', '2': 13},
    {'1': 'ERROR_CODE_ADAPTER_INSTALL_FAILED', '2': 14},
    {'1': 'ERROR_CODE_SESSION_NOT_FOUND', '2': 15},
    {'1': 'ERROR_CODE_SESSION_ALREADY_ACTIVE', '2': 16},
    {'1': 'ERROR_CODE_INVALID_ZONE', '2': 17},
    {'1': 'ERROR_CODE_DOOR_ALREADY_LOCKED', '2': 18},
    {'1': 'ERROR_CODE_DOOR_ALREADY_UNLOCKED', '2': 19},
    {'1': 'ERROR_CODE_DOOR_OPEN', '2': 20},
  ],
};

/// Descriptor for `ErrorCode`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List errorCodeDescriptor = $convert.base64Decode(
    'CglFcnJvckNvZGUSGgoWRVJST1JfQ09ERV9VTlNQRUNJRklFRBAAEh4KGkVSUk9SX0NPREVfSU'
    '5WQUxJRF9SRVFVRVNUEAESIAocRVJST1JfQ09ERV9TRVJWSUNFX05PVF9SRUFEWRACEiMKH0VS'
    'Uk9SX0NPREVfUkVHSVNUUllfVU5BVkFJTEFCTEUQAxIgChxFUlJPUl9DT0RFX0NIRUNLU1VNX0'
    '1JU01BVENIEAQSGAoURVJST1JfQ09ERV9UTFNfRVJST1IQBRIgChxFUlJPUl9DT0RFX0NPTk5F'
    'Q1RJT05fRkFJTEVEEAYSGgoWRVJST1JfQ09ERV9BVVRIX0ZBSUxFRBAHEiAKHEVSUk9SX0NPRE'
    'VfUEVSTUlTU0lPTl9ERU5JRUQQCBIYChRFUlJPUl9DT0RFX05PVF9GT1VORBAJEhYKEkVSUk9S'
    'X0NPREVfVElNRU9VVBAKEh0KGUVSUk9SX0NPREVfSU5URVJOQUxfRVJST1IQCxIgChxFUlJPUl'
    '9DT0RFX0FEQVBURVJfTk9UX0ZPVU5EEAwSKAokRVJST1JfQ09ERV9BREFQVEVSX0FMUkVBRFlf'
    'SU5TVEFMTEVEEA0SJQohRVJST1JfQ09ERV9BREFQVEVSX0lOU1RBTExfRkFJTEVEEA4SIAocRV'
    'JST1JfQ09ERV9TRVNTSU9OX05PVF9GT1VORBAPEiUKIUVSUk9SX0NPREVfU0VTU0lPTl9BTFJF'
    'QURZX0FDVElWRRAQEhsKF0VSUk9SX0NPREVfSU5WQUxJRF9aT05FEBESIgoeRVJST1JfQ09ERV'
    '9ET09SX0FMUkVBRFlfTE9DS0VEEBISJAogRVJST1JfQ09ERV9ET09SX0FMUkVBRFlfVU5MT0NL'
    'RUQQExIYChRFUlJPUl9DT0RFX0RPT1JfT1BFThAU');

@$core.Deprecated('Use errorDetailsDescriptor instead')
const ErrorDetails$json = {
  '1': 'ErrorDetails',
  '2': [
    {'1': 'code', '3': 1, '4': 1, '5': 9, '10': 'code'},
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
    {
      '1': 'details',
      '3': 3,
      '4': 3,
      '5': 11,
      '6': '.sdv.common.ErrorDetails.DetailsEntry',
      '10': 'details'
    },
    {'1': 'timestamp', '3': 4, '4': 1, '5': 3, '10': 'timestamp'},
  ],
  '3': [ErrorDetails_DetailsEntry$json],
};

@$core.Deprecated('Use errorDetailsDescriptor instead')
const ErrorDetails_DetailsEntry$json = {
  '1': 'DetailsEntry',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'value', '3': 2, '4': 1, '5': 9, '10': 'value'},
  ],
  '7': {'7': true},
};

/// Descriptor for `ErrorDetails`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List errorDetailsDescriptor = $convert.base64Decode(
    'CgxFcnJvckRldGFpbHMSEgoEY29kZRgBIAEoCVIEY29kZRIYCgdtZXNzYWdlGAIgASgJUgdtZX'
    'NzYWdlEj8KB2RldGFpbHMYAyADKAsyJS5zZHYuY29tbW9uLkVycm9yRGV0YWlscy5EZXRhaWxz'
    'RW50cnlSB2RldGFpbHMSHAoJdGltZXN0YW1wGAQgASgDUgl0aW1lc3RhbXAaOgoMRGV0YWlsc0'
    'VudHJ5EhAKA2tleRgBIAEoCVIDa2V5EhQKBXZhbHVlGAIgASgJUgV2YWx1ZToCOAE=');
