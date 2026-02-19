/// Data models for CLOUD_GATEWAY REST API responses.
///
/// Covers pairing, vehicle status, command feedback, and error handling.
library;

import 'dart:convert';

/// Response from POST /api/v1/pair.
class PairResponse {
  final String token;
  final String vin;

  const PairResponse({required this.token, required this.vin});

  factory PairResponse.fromJson(Map<String, dynamic> json) {
    return PairResponse(
      token: json['token'] as String,
      vin: json['vin'] as String,
    );
  }

  Map<String, dynamic> toJson() => {'token': token, 'vin': vin};
}

/// Vehicle status from GET /api/v1/vehicles/{vin}/status.
///
/// Fields may be null when the vehicle state is unknown.
class VehicleStatus {
  final String vin;
  final bool? isLocked;
  final bool? isDoorOpen;
  final double? speed;
  final double? latitude;
  final double? longitude;
  final bool? parkingSessionActive;
  final CommandInfo? lastCommand;
  final DateTime? updatedAt;

  const VehicleStatus({
    required this.vin,
    this.isLocked,
    this.isDoorOpen,
    this.speed,
    this.latitude,
    this.longitude,
    this.parkingSessionActive,
    this.lastCommand,
    this.updatedAt,
  });

  factory VehicleStatus.fromJson(Map<String, dynamic> json) {
    return VehicleStatus(
      vin: json['vin'] as String,
      isLocked: json['is_locked'] as bool?,
      isDoorOpen: json['is_door_open'] as bool?,
      speed: (json['speed'] as num?)?.toDouble(),
      latitude: (json['latitude'] as num?)?.toDouble(),
      longitude: (json['longitude'] as num?)?.toDouble(),
      parkingSessionActive: json['parking_session_active'] as bool?,
      lastCommand: json['last_command'] != null
          ? CommandInfo.fromJson(json['last_command'] as Map<String, dynamic>)
          : null,
      updatedAt: json['updated_at'] != null &&
              (json['updated_at'] as String).isNotEmpty
          ? DateTime.parse(json['updated_at'] as String)
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
        'vin': vin,
        'is_locked': isLocked,
        'is_door_open': isDoorOpen,
        'speed': speed,
        'latitude': latitude,
        'longitude': longitude,
        'parking_session_active': parkingSessionActive,
        'last_command': lastCommand?.toJson(),
        'updated_at': updatedAt?.toIso8601String(),
      };
}

/// Information about the most recent command in a vehicle status response.
class CommandInfo {
  final String commandId;
  final String type; // "lock" or "unlock"
  final String status; // "accepted", "success", "rejected"
  final String? result; // "SUCCESS", "REJECTED_SPEED", "REJECTED_DOOR_OPEN"

  const CommandInfo({
    required this.commandId,
    required this.type,
    required this.status,
    this.result,
  });

  factory CommandInfo.fromJson(Map<String, dynamic> json) {
    final rawResult = json['result'] as String?;
    return CommandInfo(
      commandId: json['command_id'] as String,
      type: json['type'] as String,
      status: json['status'] as String,
      result: (rawResult != null && rawResult.isNotEmpty) ? rawResult : null,
    );
  }

  Map<String, dynamic> toJson() => {
        'command_id': commandId,
        'type': type,
        'status': status,
        'result': result,
      };
}

/// Response from POST /api/v1/vehicles/{vin}/lock or /unlock.
class CommandResponse {
  final String commandId;
  final String status; // "accepted"

  const CommandResponse({required this.commandId, required this.status});

  factory CommandResponse.fromJson(Map<String, dynamic> json) {
    return CommandResponse(
      commandId: json['command_id'] as String,
      status: json['status'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
        'command_id': commandId,
        'status': status,
      };
}

/// Exception thrown when CLOUD_GATEWAY returns a non-success HTTP response.
class GatewayException implements Exception {
  final int statusCode;
  final String message;
  final String? code;

  const GatewayException({
    required this.statusCode,
    required this.message,
    this.code,
  });

  /// Create a [GatewayException] from an HTTP response status code and body.
  ///
  /// Expected error format: `{"error": "message", "code": "CODE"}`.
  factory GatewayException.fromResponse(int statusCode, String body) {
    try {
      final json = jsonDecode(body) as Map<String, dynamic>;
      return GatewayException(
        statusCode: statusCode,
        message: (json['error'] as String?) ?? 'Unknown error',
        code: json['code'] as String?,
      );
    } catch (_) {
      return GatewayException(
        statusCode: statusCode,
        message: body.isNotEmpty ? body : 'HTTP $statusCode',
      );
    }
  }

  /// User-friendly message based on the status code and error details.
  String get userMessage {
    switch (statusCode) {
      case 401:
        return 'Unauthorized — please re-pair.';
      case 403:
        return 'Wrong PIN';
      case 404:
        return 'Vehicle not found';
      default:
        return message;
    }
  }

  @override
  String toString() => 'GatewayException($statusCode): $message';
}
