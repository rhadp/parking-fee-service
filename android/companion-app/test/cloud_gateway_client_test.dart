import 'dart:convert';

import 'package:companion_app/models/models.dart';
import 'package:companion_app/services/cloud_gateway_client.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  const baseUrl = 'http://localhost:8081';
  const testVin = 'DEMO0000000000001';
  const testPin = '1234';
  const testToken = 'test-bearer-token';

  // ──────────────────────────────────────────────────────────────────────
  // Pairing: POST /api/v1/pair
  // ──────────────────────────────────────────────────────────────────────

  group('pair()', () {
    test('returns PairResponse on HTTP 200', () async {
      final mockClient = MockClient((request) async {
        expect(request.method, 'POST');
        expect(request.url.toString(), '$baseUrl/api/v1/pair');
        expect(request.headers['Content-Type'], 'application/json');

        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['vin'], testVin);
        expect(body['pin'], testPin);

        return http.Response(
          jsonEncode({'token': testToken, 'vin': testVin}),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final result = await client.pair(testVin, testPin);

      expect(result.token, testToken);
      expect(result.vin, testVin);
    });

    test('throws GatewayException on HTTP 403 (wrong PIN)', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({'error': 'incorrect pairing PIN', 'code': 'FORBIDDEN'}),
          403,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.pair(testVin, 'wrong'),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 403)
              .having((e) => e.message, 'message', 'incorrect pairing PIN')
              .having((e) => e.code, 'code', 'FORBIDDEN'),
        ),
      );
    });

    test('throws GatewayException on HTTP 404 (unknown VIN)', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({'error': 'vehicle not found', 'code': 'NOT_FOUND'}),
          404,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.pair('UNKNOWNVIN', testPin),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 404)
              .having((e) => e.message, 'message', 'vehicle not found')
              .having((e) => e.code, 'code', 'NOT_FOUND'),
        ),
      );
    });

    test('throws GatewayException on network error', () async {
      final mockClient = MockClient((request) async {
        throw Exception('Connection refused');
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.pair(testVin, testPin),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 0)
              .having((e) => e.code, 'code', 'NETWORK_ERROR'),
        ),
      );
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Lock: POST /api/v1/vehicles/{vin}/lock
  // ──────────────────────────────────────────────────────────────────────

  group('lock()', () {
    test('returns CommandResponse on HTTP 202', () async {
      const commandId = 'cmd-lock-1';
      final mockClient = MockClient((request) async {
        expect(request.method, 'POST');
        expect(
          request.url.toString(),
          '$baseUrl/api/v1/vehicles/$testVin/lock',
        );
        expect(request.headers['Authorization'], 'Bearer $testToken');

        return http.Response(
          jsonEncode({'command_id': commandId, 'status': 'accepted'}),
          202,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final result = await client.lock(testVin, testToken);

      expect(result.commandId, commandId);
      expect(result.status, 'accepted');
    });

    test('sends Authorization header (Property 1: Token-Request Consistency)',
        () async {
      String? capturedAuth;
      final mockClient = MockClient((request) async {
        capturedAuth = request.headers['Authorization'];
        return http.Response(
          jsonEncode({'command_id': 'cmd-1', 'status': 'accepted'}),
          202,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      await client.lock(testVin, testToken);

      expect(capturedAuth, 'Bearer $testToken');
    });

    test('throws GatewayException on HTTP 401', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({'error': 'unauthorized', 'code': 'UNAUTHORIZED'}),
          401,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.lock(testVin, 'bad-token'),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 401),
        ),
      );
    });

    test('throws GatewayException on network error', () async {
      final mockClient = MockClient((request) async {
        throw Exception('Connection refused');
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.lock(testVin, testToken),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 0)
              .having((e) => e.code, 'code', 'NETWORK_ERROR'),
        ),
      );
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Unlock: POST /api/v1/vehicles/{vin}/unlock
  // ──────────────────────────────────────────────────────────────────────

  group('unlock()', () {
    test('returns CommandResponse on HTTP 202', () async {
      const commandId = 'cmd-unlock-1';
      final mockClient = MockClient((request) async {
        expect(request.method, 'POST');
        expect(
          request.url.toString(),
          '$baseUrl/api/v1/vehicles/$testVin/unlock',
        );
        expect(request.headers['Authorization'], 'Bearer $testToken');

        return http.Response(
          jsonEncode({'command_id': commandId, 'status': 'accepted'}),
          202,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final result = await client.unlock(testVin, testToken);

      expect(result.commandId, commandId);
      expect(result.status, 'accepted');
    });

    test('sends Authorization header (Property 1: Token-Request Consistency)',
        () async {
      String? capturedAuth;
      final mockClient = MockClient((request) async {
        capturedAuth = request.headers['Authorization'];
        return http.Response(
          jsonEncode({'command_id': 'cmd-1', 'status': 'accepted'}),
          202,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      await client.unlock(testVin, testToken);

      expect(capturedAuth, 'Bearer $testToken');
    });

    test('throws GatewayException on HTTP 401', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({'error': 'unauthorized', 'code': 'UNAUTHORIZED'}),
          401,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.unlock(testVin, 'bad-token'),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 401),
        ),
      );
    });

    test('throws GatewayException on network error', () async {
      final mockClient = MockClient((request) async {
        throw Exception('Connection refused');
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.unlock(testVin, testToken),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 0)
              .having((e) => e.code, 'code', 'NETWORK_ERROR'),
        ),
      );
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Status: GET /api/v1/vehicles/{vin}/status
  // ──────────────────────────────────────────────────────────────────────

  group('getStatus()', () {
    test('returns VehicleStatus with all fields on HTTP 200', () async {
      final mockClient = MockClient((request) async {
        expect(request.method, 'GET');
        expect(
          request.url.toString(),
          '$baseUrl/api/v1/vehicles/$testVin/status',
        );
        expect(request.headers['Authorization'], 'Bearer $testToken');

        return http.Response(
          jsonEncode({
            'vin': testVin,
            'is_locked': true,
            'is_door_open': false,
            'speed': 0.0,
            'latitude': 48.1351,
            'longitude': 11.5820,
            'parking_session_active': false,
            'last_command': {
              'command_id': 'cmd-1',
              'type': 'lock',
              'status': 'success',
              'result': 'SUCCESS',
            },
            'updated_at': '2024-02-19T10:00:00Z',
          }),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final status = await client.getStatus(testVin, testToken);

      expect(status.vin, testVin);
      expect(status.isLocked, true);
      expect(status.isDoorOpen, false);
      expect(status.speed, 0.0);
      expect(status.latitude, 48.1351);
      expect(status.longitude, 11.5820);
      expect(status.parkingSessionActive, false);
      expect(status.lastCommand, isNotNull);
      expect(status.lastCommand!.commandId, 'cmd-1');
      expect(status.lastCommand!.type, 'lock');
      expect(status.lastCommand!.status, 'success');
      expect(status.lastCommand!.result, 'SUCCESS');
      expect(status.updatedAt, isNotNull);
      expect(status.updatedAt!.year, 2024);
    });

    test('preserves null fields when state is unknown', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({
            'vin': testVin,
            'is_locked': null,
            'is_door_open': null,
            'speed': null,
            'latitude': null,
            'longitude': null,
            'parking_session_active': null,
            'last_command': null,
            'updated_at': '',
          }),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final status = await client.getStatus(testVin, testToken);

      expect(status.vin, testVin);
      expect(status.isLocked, isNull);
      expect(status.isDoorOpen, isNull);
      expect(status.speed, isNull);
      expect(status.latitude, isNull);
      expect(status.longitude, isNull);
      expect(status.parkingSessionActive, isNull);
      expect(status.lastCommand, isNull);
      expect(status.updatedAt, isNull);
    });

    test('handles missing optional fields in response', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({
            'vin': testVin,
            'updated_at': '',
          }),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final status = await client.getStatus(testVin, testToken);

      expect(status.vin, testVin);
      expect(status.isLocked, isNull);
      expect(status.isDoorOpen, isNull);
      expect(status.speed, isNull);
      expect(status.latitude, isNull);
      expect(status.longitude, isNull);
      expect(status.parkingSessionActive, isNull);
      expect(status.lastCommand, isNull);
      expect(status.updatedAt, isNull);
    });

    test('sends Authorization header (Property 1: Token-Request Consistency)',
        () async {
      String? capturedAuth;
      final mockClient = MockClient((request) async {
        capturedAuth = request.headers['Authorization'];
        return http.Response(
          jsonEncode({
            'vin': testVin,
            'updated_at': '',
          }),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      await client.getStatus(testVin, testToken);

      expect(capturedAuth, 'Bearer $testToken');
    });

    test('throws GatewayException on HTTP 404', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({'error': 'vehicle not found', 'code': 'NOT_FOUND'}),
          404,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.getStatus('UNKNOWN', testToken),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 404),
        ),
      );
    });

    test('throws GatewayException on HTTP 401', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({'error': 'unauthorized', 'code': 'UNAUTHORIZED'}),
          401,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.getStatus(testVin, 'bad-token'),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 401),
        ),
      );
    });

    test('throws GatewayException on network error', () async {
      final mockClient = MockClient((request) async {
        throw Exception('Connection refused');
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);

      expect(
        () => client.getStatus(testVin, testToken),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 0)
              .having((e) => e.code, 'code', 'NETWORK_ERROR'),
        ),
      );
    });

    test('handles last_command with empty result string', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({
            'vin': testVin,
            'is_locked': false,
            'last_command': {
              'command_id': 'cmd-2',
              'type': 'unlock',
              'status': 'accepted',
              'result': '',
            },
            'updated_at': '2024-02-19T10:00:00Z',
          }),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final status = await client.getStatus(testVin, testToken);

      expect(status.lastCommand, isNotNull);
      expect(status.lastCommand!.commandId, 'cmd-2');
      expect(status.lastCommand!.status, 'accepted');
      // Empty result string should be treated as null.
      expect(status.lastCommand!.result, isNull);
    });

    test('handles integer speed (num → double conversion)', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({
            'vin': testVin,
            'speed': 50, // integer, not double
            'updated_at': '',
          }),
          200,
        );
      });

      final client =
          CloudGatewayClient(baseUrl: baseUrl, client: mockClient);
      final status = await client.getStatus(testVin, testToken);

      expect(status.speed, 50.0);
      expect(status.speed, isA<double>());
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Model tests
  // ──────────────────────────────────────────────────────────────────────

  group('GatewayException', () {
    test('fromResponse parses standard error JSON', () {
      final exception = GatewayException.fromResponse(
        403,
        jsonEncode({'error': 'incorrect pairing PIN', 'code': 'FORBIDDEN'}),
      );

      expect(exception.statusCode, 403);
      expect(exception.message, 'incorrect pairing PIN');
      expect(exception.code, 'FORBIDDEN');
    });

    test('fromResponse handles non-JSON body', () {
      final exception = GatewayException.fromResponse(
        500,
        'Internal Server Error',
      );

      expect(exception.statusCode, 500);
      expect(exception.message, 'Internal Server Error');
      expect(exception.code, isNull);
    });

    test('fromResponse handles empty body', () {
      final exception = GatewayException.fromResponse(502, '');

      expect(exception.statusCode, 502);
      expect(exception.message, 'HTTP 502');
    });

    test('userMessage returns friendly message for 403', () {
      const exception = GatewayException(
        statusCode: 403,
        message: 'incorrect pairing PIN',
      );
      expect(exception.userMessage, 'Wrong PIN');
    });

    test('userMessage returns friendly message for 404', () {
      const exception = GatewayException(
        statusCode: 404,
        message: 'vehicle not found',
      );
      expect(exception.userMessage, 'Vehicle not found');
    });

    test('userMessage returns friendly message for 401', () {
      const exception = GatewayException(
        statusCode: 401,
        message: 'unauthorized',
      );
      expect(exception.userMessage, 'Unauthorized — please re-pair.');
    });

    test('userMessage returns raw message for other status codes', () {
      const exception = GatewayException(
        statusCode: 500,
        message: 'internal server error',
      );
      expect(exception.userMessage, 'internal server error');
    });

    test('toString includes status code and message', () {
      const exception = GatewayException(
        statusCode: 403,
        message: 'incorrect pairing PIN',
      );
      expect(exception.toString(), 'GatewayException(403): incorrect pairing PIN');
    });
  });

  group('PairResponse', () {
    test('fromJson parses correctly', () {
      final response = PairResponse.fromJson({
        'token': 'abc123',
        'vin': testVin,
      });

      expect(response.token, 'abc123');
      expect(response.vin, testVin);
    });

    test('toJson round-trips', () {
      const original = PairResponse(token: 'abc123', vin: testVin);
      final json = original.toJson();
      final restored = PairResponse.fromJson(json);

      expect(restored.token, original.token);
      expect(restored.vin, original.vin);
    });
  });

  group('VehicleStatus', () {
    test('fromJson parses full response', () {
      final status = VehicleStatus.fromJson({
        'vin': testVin,
        'is_locked': true,
        'is_door_open': false,
        'speed': 60.5,
        'latitude': 48.1351,
        'longitude': 11.5820,
        'parking_session_active': true,
        'last_command': {
          'command_id': 'cmd-1',
          'type': 'lock',
          'status': 'success',
          'result': 'SUCCESS',
        },
        'updated_at': '2024-01-01T12:00:00Z',
      });

      expect(status.vin, testVin);
      expect(status.isLocked, true);
      expect(status.isDoorOpen, false);
      expect(status.speed, 60.5);
      expect(status.latitude, 48.1351);
      expect(status.longitude, 11.5820);
      expect(status.parkingSessionActive, true);
      expect(status.lastCommand!.commandId, 'cmd-1');
      expect(status.updatedAt!.year, 2024);
    });

    test('fromJson handles all-null optional fields', () {
      final status = VehicleStatus.fromJson({
        'vin': testVin,
        'is_locked': null,
        'is_door_open': null,
        'speed': null,
        'latitude': null,
        'longitude': null,
        'parking_session_active': null,
        'last_command': null,
        'updated_at': '',
      });

      expect(status.isLocked, isNull);
      expect(status.isDoorOpen, isNull);
      expect(status.speed, isNull);
      expect(status.latitude, isNull);
      expect(status.longitude, isNull);
      expect(status.parkingSessionActive, isNull);
      expect(status.lastCommand, isNull);
      expect(status.updatedAt, isNull);
    });

    test('toJson round-trips', () {
      final original = VehicleStatus(
        vin: testVin,
        isLocked: true,
        speed: 10.0,
        updatedAt: DateTime.utc(2024, 1, 1),
      );
      final json = original.toJson();
      final restored = VehicleStatus.fromJson(json);

      expect(restored.vin, original.vin);
      expect(restored.isLocked, original.isLocked);
      expect(restored.speed, original.speed);
    });
  });

  group('CommandResponse', () {
    test('fromJson parses correctly', () {
      final response = CommandResponse.fromJson({
        'command_id': 'cmd-1',
        'status': 'accepted',
      });

      expect(response.commandId, 'cmd-1');
      expect(response.status, 'accepted');
    });

    test('toJson round-trips', () {
      const original =
          CommandResponse(commandId: 'cmd-1', status: 'accepted');
      final json = original.toJson();
      final restored = CommandResponse.fromJson(json);

      expect(restored.commandId, original.commandId);
      expect(restored.status, original.status);
    });
  });

  group('CommandInfo', () {
    test('fromJson parses correctly', () {
      final info = CommandInfo.fromJson({
        'command_id': 'cmd-1',
        'type': 'lock',
        'status': 'success',
        'result': 'SUCCESS',
      });

      expect(info.commandId, 'cmd-1');
      expect(info.type, 'lock');
      expect(info.status, 'success');
      expect(info.result, 'SUCCESS');
    });

    test('fromJson treats empty result as null', () {
      final info = CommandInfo.fromJson({
        'command_id': 'cmd-1',
        'type': 'unlock',
        'status': 'accepted',
        'result': '',
      });

      expect(info.result, isNull);
    });

    test('fromJson handles null result', () {
      final info = CommandInfo.fromJson({
        'command_id': 'cmd-1',
        'type': 'lock',
        'status': 'accepted',
        'result': null,
      });

      expect(info.result, isNull);
    });

    test('toJson round-trips', () {
      const original = CommandInfo(
        commandId: 'cmd-1',
        type: 'lock',
        status: 'rejected',
        result: 'REJECTED_SPEED',
      );
      final json = original.toJson();
      final restored = CommandInfo.fromJson(json);

      expect(restored.commandId, original.commandId);
      expect(restored.type, original.type);
      expect(restored.status, original.status);
      expect(restored.result, original.result);
    });
  });
}
