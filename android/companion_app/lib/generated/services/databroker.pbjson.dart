// This is a generated file - do not edit.
//
// Generated from services/databroker.proto.

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

@$core.Deprecated('Use getSignalRequestDescriptor instead')
const GetSignalRequest$json = {
  '1': 'GetSignalRequest',
  '2': [
    {'1': 'signal_paths', '3': 1, '4': 3, '5': 9, '10': 'signalPaths'},
  ],
};

/// Descriptor for `GetSignalRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSignalRequestDescriptor = $convert.base64Decode(
    'ChBHZXRTaWduYWxSZXF1ZXN0EiEKDHNpZ25hbF9wYXRocxgBIAMoCVILc2lnbmFsUGF0aHM=');

@$core.Deprecated('Use getSignalResponseDescriptor instead')
const GetSignalResponse$json = {
  '1': 'GetSignalResponse',
  '2': [
    {
      '1': 'signals',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.sdv.vss.VehicleSignal',
      '10': 'signals'
    },
  ],
};

/// Descriptor for `GetSignalResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSignalResponseDescriptor = $convert.base64Decode(
    'ChFHZXRTaWduYWxSZXNwb25zZRIwCgdzaWduYWxzGAEgAygLMhYuc2R2LnZzcy5WZWhpY2xlU2'
    'lnbmFsUgdzaWduYWxz');

@$core.Deprecated('Use setSignalRequestDescriptor instead')
const SetSignalRequest$json = {
  '1': 'SetSignalRequest',
  '2': [
    {'1': 'signal_path', '3': 1, '4': 1, '5': 9, '10': 'signalPath'},
    {
      '1': 'signal',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.sdv.vss.VehicleSignal',
      '10': 'signal'
    },
  ],
};

/// Descriptor for `SetSignalRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List setSignalRequestDescriptor = $convert.base64Decode(
    'ChBTZXRTaWduYWxSZXF1ZXN0Eh8KC3NpZ25hbF9wYXRoGAEgASgJUgpzaWduYWxQYXRoEi4KBn'
    'NpZ25hbBgCIAEoCzIWLnNkdi52c3MuVmVoaWNsZVNpZ25hbFIGc2lnbmFs');

@$core.Deprecated('Use setSignalResponseDescriptor instead')
const SetSignalResponse$json = {
  '1': 'SetSignalResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
  ],
};

/// Descriptor for `SetSignalResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List setSignalResponseDescriptor = $convert.base64Decode(
    'ChFTZXRTaWduYWxSZXNwb25zZRIYCgdzdWNjZXNzGAEgASgIUgdzdWNjZXNzEiMKDWVycm9yX2'
    '1lc3NhZ2UYAiABKAlSDGVycm9yTWVzc2FnZQ==');

@$core.Deprecated('Use subscribeRequestDescriptor instead')
const SubscribeRequest$json = {
  '1': 'SubscribeRequest',
  '2': [
    {'1': 'signal_paths', '3': 1, '4': 3, '5': 9, '10': 'signalPaths'},
  ],
};

/// Descriptor for `SubscribeRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List subscribeRequestDescriptor = $convert.base64Decode(
    'ChBTdWJzY3JpYmVSZXF1ZXN0EiEKDHNpZ25hbF9wYXRocxgBIAMoCVILc2lnbmFsUGF0aHM=');

@$core.Deprecated('Use subscribeResponseDescriptor instead')
const SubscribeResponse$json = {
  '1': 'SubscribeResponse',
  '2': [
    {'1': 'signal_path', '3': 1, '4': 1, '5': 9, '10': 'signalPath'},
    {
      '1': 'signal',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.sdv.vss.VehicleSignal',
      '10': 'signal'
    },
  ],
};

/// Descriptor for `SubscribeResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List subscribeResponseDescriptor = $convert.base64Decode(
    'ChFTdWJzY3JpYmVSZXNwb25zZRIfCgtzaWduYWxfcGF0aBgBIAEoCVIKc2lnbmFsUGF0aBIuCg'
    'ZzaWduYWwYAiABKAsyFi5zZHYudnNzLlZlaGljbGVTaWduYWxSBnNpZ25hbA==');
