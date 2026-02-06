// This is a generated file - do not edit.
//
// Generated from services/locking_service.proto.

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

@$core.Deprecated('Use doorDescriptor instead')
const Door$json = {
  '1': 'Door',
  '2': [
    {'1': 'DOOR_UNKNOWN', '2': 0},
    {'1': 'DOOR_DRIVER', '2': 1},
    {'1': 'DOOR_PASSENGER', '2': 2},
    {'1': 'DOOR_REAR_LEFT', '2': 3},
    {'1': 'DOOR_REAR_RIGHT', '2': 4},
    {'1': 'DOOR_ALL', '2': 5},
  ],
};

/// Descriptor for `Door`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List doorDescriptor = $convert.base64Decode(
    'CgREb29yEhAKDERPT1JfVU5LTk9XThAAEg8KC0RPT1JfRFJJVkVSEAESEgoORE9PUl9QQVNTRU'
    '5HRVIQAhISCg5ET09SX1JFQVJfTEVGVBADEhMKD0RPT1JfUkVBUl9SSUdIVBAEEgwKCERPT1Jf'
    'QUxMEAU=');

@$core.Deprecated('Use lockRequestDescriptor instead')
const LockRequest$json = {
  '1': 'LockRequest',
  '2': [
    {
      '1': 'door',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.locking.Door',
      '10': 'door'
    },
    {'1': 'command_id', '3': 2, '4': 1, '5': 9, '10': 'commandId'},
    {'1': 'auth_token', '3': 3, '4': 1, '5': 9, '10': 'authToken'},
  ],
};

/// Descriptor for `LockRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List lockRequestDescriptor = $convert.base64Decode(
    'CgtMb2NrUmVxdWVzdBIuCgRkb29yGAEgASgOMhouc2R2LnNlcnZpY2VzLmxvY2tpbmcuRG9vcl'
    'IEZG9vchIdCgpjb21tYW5kX2lkGAIgASgJUgljb21tYW5kSWQSHQoKYXV0aF90b2tlbhgDIAEo'
    'CVIJYXV0aFRva2Vu');

@$core.Deprecated('Use lockResponseDescriptor instead')
const LockResponse$json = {
  '1': 'LockResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'command_id', '3': 3, '4': 1, '5': 9, '10': 'commandId'},
  ],
};

/// Descriptor for `LockResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List lockResponseDescriptor = $convert.base64Decode(
    'CgxMb2NrUmVzcG9uc2USGAoHc3VjY2VzcxgBIAEoCFIHc3VjY2VzcxIjCg1lcnJvcl9tZXNzYW'
    'dlGAIgASgJUgxlcnJvck1lc3NhZ2USHQoKY29tbWFuZF9pZBgDIAEoCVIJY29tbWFuZElk');

@$core.Deprecated('Use unlockRequestDescriptor instead')
const UnlockRequest$json = {
  '1': 'UnlockRequest',
  '2': [
    {
      '1': 'door',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.locking.Door',
      '10': 'door'
    },
    {'1': 'command_id', '3': 2, '4': 1, '5': 9, '10': 'commandId'},
    {'1': 'auth_token', '3': 3, '4': 1, '5': 9, '10': 'authToken'},
  ],
};

/// Descriptor for `UnlockRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List unlockRequestDescriptor = $convert.base64Decode(
    'Cg1VbmxvY2tSZXF1ZXN0Ei4KBGRvb3IYASABKA4yGi5zZHYuc2VydmljZXMubG9ja2luZy5Eb2'
    '9yUgRkb29yEh0KCmNvbW1hbmRfaWQYAiABKAlSCWNvbW1hbmRJZBIdCgphdXRoX3Rva2VuGAMg'
    'ASgJUglhdXRoVG9rZW4=');

@$core.Deprecated('Use unlockResponseDescriptor instead')
const UnlockResponse$json = {
  '1': 'UnlockResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
    {'1': 'command_id', '3': 3, '4': 1, '5': 9, '10': 'commandId'},
  ],
};

/// Descriptor for `UnlockResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List unlockResponseDescriptor = $convert.base64Decode(
    'Cg5VbmxvY2tSZXNwb25zZRIYCgdzdWNjZXNzGAEgASgIUgdzdWNjZXNzEiMKDWVycm9yX21lc3'
    'NhZ2UYAiABKAlSDGVycm9yTWVzc2FnZRIdCgpjb21tYW5kX2lkGAMgASgJUgljb21tYW5kSWQ=');

@$core.Deprecated('Use getLockStateRequestDescriptor instead')
const GetLockStateRequest$json = {
  '1': 'GetLockStateRequest',
  '2': [
    {
      '1': 'door',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.locking.Door',
      '10': 'door'
    },
  ],
};

/// Descriptor for `GetLockStateRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getLockStateRequestDescriptor = $convert.base64Decode(
    'ChNHZXRMb2NrU3RhdGVSZXF1ZXN0Ei4KBGRvb3IYASABKA4yGi5zZHYuc2VydmljZXMubG9ja2'
    'luZy5Eb29yUgRkb29y');

@$core.Deprecated('Use getLockStateResponseDescriptor instead')
const GetLockStateResponse$json = {
  '1': 'GetLockStateResponse',
  '2': [
    {
      '1': 'door',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.locking.Door',
      '10': 'door'
    },
    {'1': 'is_locked', '3': 2, '4': 1, '5': 8, '10': 'isLocked'},
    {'1': 'is_open', '3': 3, '4': 1, '5': 8, '10': 'isOpen'},
  ],
};

/// Descriptor for `GetLockStateResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getLockStateResponseDescriptor = $convert.base64Decode(
    'ChRHZXRMb2NrU3RhdGVSZXNwb25zZRIuCgRkb29yGAEgASgOMhouc2R2LnNlcnZpY2VzLmxvY2'
    'tpbmcuRG9vclIEZG9vchIbCglpc19sb2NrZWQYAiABKAhSCGlzTG9ja2VkEhcKB2lzX29wZW4Y'
    'AyABKAhSBmlzT3Blbg==');
