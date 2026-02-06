// This is a generated file - do not edit.
//
// Generated from services/locking_service.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

class Door extends $pb.ProtobufEnum {
  static const Door DOOR_UNKNOWN =
      Door._(0, _omitEnumNames ? '' : 'DOOR_UNKNOWN');
  static const Door DOOR_DRIVER =
      Door._(1, _omitEnumNames ? '' : 'DOOR_DRIVER');
  static const Door DOOR_PASSENGER =
      Door._(2, _omitEnumNames ? '' : 'DOOR_PASSENGER');
  static const Door DOOR_REAR_LEFT =
      Door._(3, _omitEnumNames ? '' : 'DOOR_REAR_LEFT');
  static const Door DOOR_REAR_RIGHT =
      Door._(4, _omitEnumNames ? '' : 'DOOR_REAR_RIGHT');
  static const Door DOOR_ALL = Door._(5, _omitEnumNames ? '' : 'DOOR_ALL');

  static const $core.List<Door> values = <Door>[
    DOOR_UNKNOWN,
    DOOR_DRIVER,
    DOOR_PASSENGER,
    DOOR_REAR_LEFT,
    DOOR_REAR_RIGHT,
    DOOR_ALL,
  ];

  static final $core.List<Door?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 5);
  static Door? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const Door._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
