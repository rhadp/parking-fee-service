/// REST client for the CLOUD_GATEWAY API.
///
/// Provides methods for vehicle pairing, lock/unlock commands, and status
/// retrieval. Accepts an injectable [http.Client] for testability.
library;

import 'dart:convert';

import 'package:http/http.dart' as http;

import '../models/models.dart';

/// HTTP client wrapper for the CLOUD_GATEWAY REST API.
///
/// All protected endpoints (lock, unlock, status) require a bearer [token]
/// obtained through the [pair] method.
class CloudGatewayClient {
  /// Base URL for the CLOUD_GATEWAY (e.g. `http://10.0.2.2:8081`).
  final String baseUrl;

  final http.Client _client;

  /// Creates a [CloudGatewayClient].
  ///
  /// If [client] is not provided, a default [http.Client] is created.
  CloudGatewayClient({
    required this.baseUrl,
    http.Client? client,
  }) : _client = client ?? http.Client();

  /// Pair with a vehicle using VIN and PIN.
  ///
  /// Calls `POST /api/v1/pair` with `{vin, pin}`.
  /// Returns a [PairResponse] containing the bearer token and VIN on success.
  /// Throws [GatewayException] on HTTP 403 (wrong PIN), 404 (unknown VIN),
  /// or other non-200 responses.
  Future<PairResponse> pair(String vin, String pin) async {
    final http.Response response;
    try {
      response = await _client.post(
        Uri.parse('$baseUrl/api/v1/pair'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'vin': vin, 'pin': pin}),
      );
    } catch (e) {
      throw GatewayException(
        statusCode: 0,
        message: 'Connection error: $e',
        code: 'NETWORK_ERROR',
      );
    }

    if (response.statusCode == 200) {
      return PairResponse.fromJson(
        jsonDecode(response.body) as Map<String, dynamic>,
      );
    }
    throw GatewayException.fromResponse(response.statusCode, response.body);
  }

  /// Send a lock command for the specified vehicle.
  ///
  /// Calls `POST /api/v1/vehicles/{vin}/lock` with the bearer token.
  /// Returns a [CommandResponse] with the command ID on HTTP 202.
  /// Throws [GatewayException] on non-202 responses.
  Future<CommandResponse> lock(String vin, String token) async {
    final http.Response response;
    try {
      response = await _client.post(
        Uri.parse('$baseUrl/api/v1/vehicles/$vin/lock'),
        headers: {'Authorization': 'Bearer $token'},
      );
    } catch (e) {
      throw GatewayException(
        statusCode: 0,
        message: 'Connection error: $e',
        code: 'NETWORK_ERROR',
      );
    }

    if (response.statusCode == 202) {
      return CommandResponse.fromJson(
        jsonDecode(response.body) as Map<String, dynamic>,
      );
    }
    throw GatewayException.fromResponse(response.statusCode, response.body);
  }

  /// Send an unlock command for the specified vehicle.
  ///
  /// Calls `POST /api/v1/vehicles/{vin}/unlock` with the bearer token.
  /// Returns a [CommandResponse] with the command ID on HTTP 202.
  /// Throws [GatewayException] on non-202 responses.
  Future<CommandResponse> unlock(String vin, String token) async {
    final http.Response response;
    try {
      response = await _client.post(
        Uri.parse('$baseUrl/api/v1/vehicles/$vin/unlock'),
        headers: {'Authorization': 'Bearer $token'},
      );
    } catch (e) {
      throw GatewayException(
        statusCode: 0,
        message: 'Connection error: $e',
        code: 'NETWORK_ERROR',
      );
    }

    if (response.statusCode == 202) {
      return CommandResponse.fromJson(
        jsonDecode(response.body) as Map<String, dynamic>,
      );
    }
    throw GatewayException.fromResponse(response.statusCode, response.body);
  }

  /// Get the current status of a vehicle.
  ///
  /// Calls `GET /api/v1/vehicles/{vin}/status` with the bearer token.
  /// Returns a [VehicleStatus] on HTTP 200.
  /// Throws [GatewayException] on non-200 responses.
  Future<VehicleStatus> getStatus(String vin, String token) async {
    final http.Response response;
    try {
      response = await _client.get(
        Uri.parse('$baseUrl/api/v1/vehicles/$vin/status'),
        headers: {'Authorization': 'Bearer $token'},
      );
    } catch (e) {
      throw GatewayException(
        statusCode: 0,
        message: 'Connection error: $e',
        code: 'NETWORK_ERROR',
      );
    }

    if (response.statusCode == 200) {
      return VehicleStatus.fromJson(
        jsonDecode(response.body) as Map<String, dynamic>,
      );
    }
    throw GatewayException.fromResponse(response.statusCode, response.body);
  }
}
