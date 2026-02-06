// This is a generated file - do not edit.
//
// Generated from services/update_service.proto.

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

@$core.Deprecated('Use adapterStateDescriptor instead')
const AdapterState$json = {
  '1': 'AdapterState',
  '2': [
    {'1': 'ADAPTER_STATE_UNKNOWN', '2': 0},
    {'1': 'ADAPTER_STATE_DOWNLOADING', '2': 1},
    {'1': 'ADAPTER_STATE_INSTALLING', '2': 2},
    {'1': 'ADAPTER_STATE_RUNNING', '2': 3},
    {'1': 'ADAPTER_STATE_STOPPED', '2': 4},
    {'1': 'ADAPTER_STATE_ERROR', '2': 5},
  ],
};

/// Descriptor for `AdapterState`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List adapterStateDescriptor = $convert.base64Decode(
    'CgxBZGFwdGVyU3RhdGUSGQoVQURBUFRFUl9TVEFURV9VTktOT1dOEAASHQoZQURBUFRFUl9TVE'
    'FURV9ET1dOTE9BRElORxABEhwKGEFEQVBURVJfU1RBVEVfSU5TVEFMTElORxACEhkKFUFEQVBU'
    'RVJfU1RBVEVfUlVOTklORxADEhkKFUFEQVBURVJfU1RBVEVfU1RPUFBFRBAEEhcKE0FEQVBURV'
    'JfU1RBVEVfRVJST1IQBQ==');

@$core.Deprecated('Use adapterInfoDescriptor instead')
const AdapterInfo$json = {
  '1': 'AdapterInfo',
  '2': [
    {'1': 'adapter_id', '3': 1, '4': 1, '5': 9, '10': 'adapterId'},
    {'1': 'image_ref', '3': 2, '4': 1, '5': 9, '10': 'imageRef'},
    {'1': 'version', '3': 3, '4': 1, '5': 9, '10': 'version'},
    {
      '1': 'state',
      '3': 4,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.update.AdapterState',
      '10': 'state'
    },
    {'1': 'error_message', '3': 5, '4': 1, '5': 9, '10': 'errorMessage'},
  ],
};

/// Descriptor for `AdapterInfo`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List adapterInfoDescriptor = $convert.base64Decode(
    'CgtBZGFwdGVySW5mbxIdCgphZGFwdGVyX2lkGAEgASgJUglhZGFwdGVySWQSGwoJaW1hZ2Vfcm'
    'VmGAIgASgJUghpbWFnZVJlZhIYCgd2ZXJzaW9uGAMgASgJUgd2ZXJzaW9uEjcKBXN0YXRlGAQg'
    'ASgOMiEuc2R2LnNlcnZpY2VzLnVwZGF0ZS5BZGFwdGVyU3RhdGVSBXN0YXRlEiMKDWVycm9yX2'
    '1lc3NhZ2UYBSABKAlSDGVycm9yTWVzc2FnZQ==');

@$core.Deprecated('Use installAdapterRequestDescriptor instead')
const InstallAdapterRequest$json = {
  '1': 'InstallAdapterRequest',
  '2': [
    {'1': 'image_ref', '3': 1, '4': 1, '5': 9, '10': 'imageRef'},
    {'1': 'checksum', '3': 2, '4': 1, '5': 9, '10': 'checksum'},
  ],
};

/// Descriptor for `InstallAdapterRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List installAdapterRequestDescriptor = $convert.base64Decode(
    'ChVJbnN0YWxsQWRhcHRlclJlcXVlc3QSGwoJaW1hZ2VfcmVmGAEgASgJUghpbWFnZVJlZhIaCg'
    'hjaGVja3N1bRgCIAEoCVIIY2hlY2tzdW0=');

@$core.Deprecated('Use installAdapterResponseDescriptor instead')
const InstallAdapterResponse$json = {
  '1': 'InstallAdapterResponse',
  '2': [
    {'1': 'job_id', '3': 1, '4': 1, '5': 9, '10': 'jobId'},
    {'1': 'adapter_id', '3': 2, '4': 1, '5': 9, '10': 'adapterId'},
    {
      '1': 'state',
      '3': 3,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.update.AdapterState',
      '10': 'state'
    },
  ],
};

/// Descriptor for `InstallAdapterResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List installAdapterResponseDescriptor = $convert.base64Decode(
    'ChZJbnN0YWxsQWRhcHRlclJlc3BvbnNlEhUKBmpvYl9pZBgBIAEoCVIFam9iSWQSHQoKYWRhcH'
    'Rlcl9pZBgCIAEoCVIJYWRhcHRlcklkEjcKBXN0YXRlGAMgASgOMiEuc2R2LnNlcnZpY2VzLnVw'
    'ZGF0ZS5BZGFwdGVyU3RhdGVSBXN0YXRl');

@$core.Deprecated('Use uninstallAdapterRequestDescriptor instead')
const UninstallAdapterRequest$json = {
  '1': 'UninstallAdapterRequest',
  '2': [
    {'1': 'adapter_id', '3': 1, '4': 1, '5': 9, '10': 'adapterId'},
  ],
};

/// Descriptor for `UninstallAdapterRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List uninstallAdapterRequestDescriptor =
    $convert.base64Decode(
        'ChdVbmluc3RhbGxBZGFwdGVyUmVxdWVzdBIdCgphZGFwdGVyX2lkGAEgASgJUglhZGFwdGVySW'
        'Q=');

@$core.Deprecated('Use uninstallAdapterResponseDescriptor instead')
const UninstallAdapterResponse$json = {
  '1': 'UninstallAdapterResponse',
  '2': [
    {'1': 'success', '3': 1, '4': 1, '5': 8, '10': 'success'},
    {'1': 'error_message', '3': 2, '4': 1, '5': 9, '10': 'errorMessage'},
  ],
};

/// Descriptor for `UninstallAdapterResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List uninstallAdapterResponseDescriptor =
    $convert.base64Decode(
        'ChhVbmluc3RhbGxBZGFwdGVyUmVzcG9uc2USGAoHc3VjY2VzcxgBIAEoCFIHc3VjY2VzcxIjCg'
        '1lcnJvcl9tZXNzYWdlGAIgASgJUgxlcnJvck1lc3NhZ2U=');

@$core.Deprecated('Use listAdaptersRequestDescriptor instead')
const ListAdaptersRequest$json = {
  '1': 'ListAdaptersRequest',
};

/// Descriptor for `ListAdaptersRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listAdaptersRequestDescriptor =
    $convert.base64Decode('ChNMaXN0QWRhcHRlcnNSZXF1ZXN0');

@$core.Deprecated('Use listAdaptersResponseDescriptor instead')
const ListAdaptersResponse$json = {
  '1': 'ListAdaptersResponse',
  '2': [
    {
      '1': 'adapters',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.sdv.services.update.AdapterInfo',
      '10': 'adapters'
    },
  ],
};

/// Descriptor for `ListAdaptersResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listAdaptersResponseDescriptor = $convert.base64Decode(
    'ChRMaXN0QWRhcHRlcnNSZXNwb25zZRI8CghhZGFwdGVycxgBIAMoCzIgLnNkdi5zZXJ2aWNlcy'
    '51cGRhdGUuQWRhcHRlckluZm9SCGFkYXB0ZXJz');

@$core.Deprecated('Use watchAdapterStatesRequestDescriptor instead')
const WatchAdapterStatesRequest$json = {
  '1': 'WatchAdapterStatesRequest',
  '2': [
    {'1': 'adapter_ids', '3': 1, '4': 3, '5': 9, '10': 'adapterIds'},
  ],
};

/// Descriptor for `WatchAdapterStatesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List watchAdapterStatesRequestDescriptor =
    $convert.base64Decode(
        'ChlXYXRjaEFkYXB0ZXJTdGF0ZXNSZXF1ZXN0Eh8KC2FkYXB0ZXJfaWRzGAEgAygJUgphZGFwdG'
        'VySWRz');

@$core.Deprecated('Use adapterStateEventDescriptor instead')
const AdapterStateEvent$json = {
  '1': 'AdapterStateEvent',
  '2': [
    {'1': 'adapter_id', '3': 1, '4': 1, '5': 9, '10': 'adapterId'},
    {
      '1': 'old_state',
      '3': 2,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.update.AdapterState',
      '10': 'oldState'
    },
    {
      '1': 'new_state',
      '3': 3,
      '4': 1,
      '5': 14,
      '6': '.sdv.services.update.AdapterState',
      '10': 'newState'
    },
    {'1': 'error_message', '3': 4, '4': 1, '5': 9, '10': 'errorMessage'},
  ],
};

/// Descriptor for `AdapterStateEvent`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List adapterStateEventDescriptor = $convert.base64Decode(
    'ChFBZGFwdGVyU3RhdGVFdmVudBIdCgphZGFwdGVyX2lkGAEgASgJUglhZGFwdGVySWQSPgoJb2'
    'xkX3N0YXRlGAIgASgOMiEuc2R2LnNlcnZpY2VzLnVwZGF0ZS5BZGFwdGVyU3RhdGVSCG9sZFN0'
    'YXRlEj4KCW5ld19zdGF0ZRgDIAEoDjIhLnNkdi5zZXJ2aWNlcy51cGRhdGUuQWRhcHRlclN0YX'
    'RlUghuZXdTdGF0ZRIjCg1lcnJvcl9tZXNzYWdlGAQgASgJUgxlcnJvck1lc3NhZ2U=');
