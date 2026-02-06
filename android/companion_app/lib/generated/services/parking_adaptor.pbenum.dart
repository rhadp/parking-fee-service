// This is a generated file - do not edit.
//
// Generated from services/parking_adaptor.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

/// Session state enumeration
class SessionState extends $pb.ProtobufEnum {
  static const SessionState SESSION_STATE_NONE =
      SessionState._(0, _omitEnumNames ? '' : 'SESSION_STATE_NONE');
  static const SessionState SESSION_STATE_STARTING =
      SessionState._(1, _omitEnumNames ? '' : 'SESSION_STATE_STARTING');
  static const SessionState SESSION_STATE_ACTIVE =
      SessionState._(2, _omitEnumNames ? '' : 'SESSION_STATE_ACTIVE');
  static const SessionState SESSION_STATE_STOPPING =
      SessionState._(3, _omitEnumNames ? '' : 'SESSION_STATE_STOPPING');
  static const SessionState SESSION_STATE_STOPPED =
      SessionState._(4, _omitEnumNames ? '' : 'SESSION_STATE_STOPPED');
  static const SessionState SESSION_STATE_ERROR =
      SessionState._(5, _omitEnumNames ? '' : 'SESSION_STATE_ERROR');

  static const $core.List<SessionState> values = <SessionState>[
    SESSION_STATE_NONE,
    SESSION_STATE_STARTING,
    SESSION_STATE_ACTIVE,
    SESSION_STATE_STOPPING,
    SESSION_STATE_STOPPED,
    SESSION_STATE_ERROR,
  ];

  static final $core.List<SessionState?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 5);
  static SessionState? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const SessionState._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
