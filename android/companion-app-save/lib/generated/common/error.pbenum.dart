// This is a generated file - do not edit.
//
// Generated from common/error.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

/// ErrorCode enum provides type-safe error codes for use in services.
/// Services should use these codes in the ErrorDetails.code field.
class ErrorCode extends $pb.ProtobufEnum {
  /// Default/unknown error
  static const ErrorCode ERROR_CODE_UNSPECIFIED =
      ErrorCode._(0, _omitEnumNames ? '' : 'ERROR_CODE_UNSPECIFIED');

  /// Invalid request or proto message
  static const ErrorCode ERROR_CODE_INVALID_REQUEST =
      ErrorCode._(1, _omitEnumNames ? '' : 'ERROR_CODE_INVALID_REQUEST');

  /// Service is not ready to handle requests
  static const ErrorCode ERROR_CODE_SERVICE_NOT_READY =
      ErrorCode._(2, _omitEnumNames ? '' : 'ERROR_CODE_SERVICE_NOT_READY');

  /// Container registry is unavailable
  static const ErrorCode ERROR_CODE_REGISTRY_UNAVAILABLE =
      ErrorCode._(3, _omitEnumNames ? '' : 'ERROR_CODE_REGISTRY_UNAVAILABLE');

  /// Container image checksum does not match expected value
  static const ErrorCode ERROR_CODE_CHECKSUM_MISMATCH =
      ErrorCode._(4, _omitEnumNames ? '' : 'ERROR_CODE_CHECKSUM_MISMATCH');

  /// TLS handshake or certificate error
  static const ErrorCode ERROR_CODE_TLS_ERROR =
      ErrorCode._(5, _omitEnumNames ? '' : 'ERROR_CODE_TLS_ERROR');

  /// Socket or network connection failed
  static const ErrorCode ERROR_CODE_CONNECTION_FAILED =
      ErrorCode._(6, _omitEnumNames ? '' : 'ERROR_CODE_CONNECTION_FAILED');

  /// Authentication failed
  static const ErrorCode ERROR_CODE_AUTH_FAILED =
      ErrorCode._(7, _omitEnumNames ? '' : 'ERROR_CODE_AUTH_FAILED');

  /// Permission denied for the requested operation
  static const ErrorCode ERROR_CODE_PERMISSION_DENIED =
      ErrorCode._(8, _omitEnumNames ? '' : 'ERROR_CODE_PERMISSION_DENIED');

  /// Requested resource was not found
  static const ErrorCode ERROR_CODE_NOT_FOUND =
      ErrorCode._(9, _omitEnumNames ? '' : 'ERROR_CODE_NOT_FOUND');

  /// Operation timed out
  static const ErrorCode ERROR_CODE_TIMEOUT =
      ErrorCode._(10, _omitEnumNames ? '' : 'ERROR_CODE_TIMEOUT');

  /// Internal server error
  static const ErrorCode ERROR_CODE_INTERNAL_ERROR =
      ErrorCode._(11, _omitEnumNames ? '' : 'ERROR_CODE_INTERNAL_ERROR');

  /// Adapter-specific errors
  static const ErrorCode ERROR_CODE_ADAPTER_NOT_FOUND =
      ErrorCode._(12, _omitEnumNames ? '' : 'ERROR_CODE_ADAPTER_NOT_FOUND');
  static const ErrorCode ERROR_CODE_ADAPTER_ALREADY_INSTALLED = ErrorCode._(
      13, _omitEnumNames ? '' : 'ERROR_CODE_ADAPTER_ALREADY_INSTALLED');
  static const ErrorCode ERROR_CODE_ADAPTER_INSTALL_FAILED = ErrorCode._(
      14, _omitEnumNames ? '' : 'ERROR_CODE_ADAPTER_INSTALL_FAILED');

  /// Parking session errors
  static const ErrorCode ERROR_CODE_SESSION_NOT_FOUND =
      ErrorCode._(15, _omitEnumNames ? '' : 'ERROR_CODE_SESSION_NOT_FOUND');
  static const ErrorCode ERROR_CODE_SESSION_ALREADY_ACTIVE = ErrorCode._(
      16, _omitEnumNames ? '' : 'ERROR_CODE_SESSION_ALREADY_ACTIVE');
  static const ErrorCode ERROR_CODE_INVALID_ZONE =
      ErrorCode._(17, _omitEnumNames ? '' : 'ERROR_CODE_INVALID_ZONE');

  /// Locking service errors
  static const ErrorCode ERROR_CODE_DOOR_ALREADY_LOCKED =
      ErrorCode._(18, _omitEnumNames ? '' : 'ERROR_CODE_DOOR_ALREADY_LOCKED');
  static const ErrorCode ERROR_CODE_DOOR_ALREADY_UNLOCKED =
      ErrorCode._(19, _omitEnumNames ? '' : 'ERROR_CODE_DOOR_ALREADY_UNLOCKED');
  static const ErrorCode ERROR_CODE_DOOR_OPEN =
      ErrorCode._(20, _omitEnumNames ? '' : 'ERROR_CODE_DOOR_OPEN');

  static const $core.List<ErrorCode> values = <ErrorCode>[
    ERROR_CODE_UNSPECIFIED,
    ERROR_CODE_INVALID_REQUEST,
    ERROR_CODE_SERVICE_NOT_READY,
    ERROR_CODE_REGISTRY_UNAVAILABLE,
    ERROR_CODE_CHECKSUM_MISMATCH,
    ERROR_CODE_TLS_ERROR,
    ERROR_CODE_CONNECTION_FAILED,
    ERROR_CODE_AUTH_FAILED,
    ERROR_CODE_PERMISSION_DENIED,
    ERROR_CODE_NOT_FOUND,
    ERROR_CODE_TIMEOUT,
    ERROR_CODE_INTERNAL_ERROR,
    ERROR_CODE_ADAPTER_NOT_FOUND,
    ERROR_CODE_ADAPTER_ALREADY_INSTALLED,
    ERROR_CODE_ADAPTER_INSTALL_FAILED,
    ERROR_CODE_SESSION_NOT_FOUND,
    ERROR_CODE_SESSION_ALREADY_ACTIVE,
    ERROR_CODE_INVALID_ZONE,
    ERROR_CODE_DOOR_ALREADY_LOCKED,
    ERROR_CODE_DOOR_ALREADY_UNLOCKED,
    ERROR_CODE_DOOR_OPEN,
  ];

  static final $core.List<ErrorCode?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 20);
  static ErrorCode? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const ErrorCode._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
