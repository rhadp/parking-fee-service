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

/// AdapterState represents the lifecycle state of an adapter
class AdapterState extends $pb.ProtobufEnum {
  static const AdapterState ADAPTER_STATE_UNKNOWN =
      AdapterState._(0, _omitEnumNames ? '' : 'ADAPTER_STATE_UNKNOWN');
  static const AdapterState ADAPTER_STATE_DOWNLOADING =
      AdapterState._(1, _omitEnumNames ? '' : 'ADAPTER_STATE_DOWNLOADING');
  static const AdapterState ADAPTER_STATE_INSTALLING =
      AdapterState._(2, _omitEnumNames ? '' : 'ADAPTER_STATE_INSTALLING');
  static const AdapterState ADAPTER_STATE_RUNNING =
      AdapterState._(3, _omitEnumNames ? '' : 'ADAPTER_STATE_RUNNING');
  static const AdapterState ADAPTER_STATE_STOPPED =
      AdapterState._(4, _omitEnumNames ? '' : 'ADAPTER_STATE_STOPPED');
  static const AdapterState ADAPTER_STATE_ERROR =
      AdapterState._(5, _omitEnumNames ? '' : 'ADAPTER_STATE_ERROR');

  static const $core.List<AdapterState> values = <AdapterState>[
    ADAPTER_STATE_UNKNOWN,
    ADAPTER_STATE_DOWNLOADING,
    ADAPTER_STATE_INSTALLING,
    ADAPTER_STATE_RUNNING,
    ADAPTER_STATE_STOPPED,
    ADAPTER_STATE_ERROR,
  ];

  static final $core.List<AdapterState?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 5);
  static AdapterState? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const AdapterState._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
