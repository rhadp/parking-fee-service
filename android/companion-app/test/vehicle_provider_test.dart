import 'dart:convert';

import 'package:companion_app/models/models.dart';
import 'package:companion_app/providers/vehicle_provider.dart';
import 'package:companion_app/services/cloud_gateway_client.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  const baseUrl = 'http://localhost:8081';
  const testVin = 'DEMO0000000000001';
  const testPin = '1234';
  const testToken = 'test-bearer-token';

  /// Helper: create a [VehicleProvider] with the given mock HTTP handler.
  Future<VehicleProvider> createProvider(
    MockClientHandler handler, {
    Map<String, Object>? initialPrefs,
  }) async {
    SharedPreferences.setMockInitialValues(initialPrefs ?? {});
    final prefs = await SharedPreferences.getInstance();
    final client = CloudGatewayClient(
      baseUrl: baseUrl,
      client: MockClient(handler),
    );
    return VehicleProvider(client: client, prefs: prefs);
  }

  /// A standard full-status JSON response.
  String fullStatusJson({
    String vin = testVin,
    bool isLocked = true,
    bool isDoorOpen = false,
    double speed = 0.0,
    String? commandId,
    String? commandType,
    String? commandStatus,
    String? commandResult,
  }) {
    final json = <String, dynamic>{
      'vin': vin,
      'is_locked': isLocked,
      'is_door_open': isDoorOpen,
      'speed': speed,
      'latitude': 48.1351,
      'longitude': 11.5820,
      'parking_session_active': false,
      'updated_at': '2024-02-19T10:00:00Z',
    };
    if (commandId != null) {
      json['last_command'] = {
        'command_id': commandId,
        'type': commandType ?? 'lock',
        'status': commandStatus ?? 'accepted',
        'result': commandResult ?? '',
      };
    }
    return jsonEncode(json);
  }

  // ──────────────────────────────────────────────────────────────────────
  // Pairing
  // ──────────────────────────────────────────────────────────────────────

  group('pair()', () {
    test('sets isPaired and persists token+VIN on success', () async {
      final provider = await createProvider((request) async {
        return http.Response(
          jsonEncode({'token': testToken, 'vin': testVin}),
          200,
        );
      });

      expect(provider.isPaired, isFalse);

      await provider.pair(testVin, testPin);

      expect(provider.isPaired, isTrue);
      expect(provider.vin, testVin);
      expect(provider.token, testToken);

      // Property 3: Token Persistence Round-Trip — verify prefs are set.
      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getString('vin'), testVin);
      expect(prefs.getString('token'), testToken);
    });

    test('notifies listeners on successful pair', () async {
      final provider = await createProvider((request) async {
        return http.Response(
          jsonEncode({'token': testToken, 'vin': testVin}),
          200,
        );
      });

      var notified = false;
      provider.addListener(() => notified = true);

      await provider.pair(testVin, testPin);

      expect(notified, isTrue);
    });

    test('rethrows GatewayException on HTTP 403 (wrong PIN)', () async {
      final provider = await createProvider((request) async {
        return http.Response(
          jsonEncode({'error': 'incorrect pairing PIN', 'code': 'FORBIDDEN'}),
          403,
        );
      });

      expect(
        () => provider.pair(testVin, 'wrong'),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 403),
        ),
      );
      expect(provider.isPaired, isFalse);
    });

    test('rethrows GatewayException on HTTP 404 (unknown VIN)', () async {
      final provider = await createProvider((request) async {
        return http.Response(
          jsonEncode({'error': 'vehicle not found', 'code': 'NOT_FOUND'}),
          404,
        );
      });

      expect(
        () => provider.pair('UNKNOWN', testPin),
        throwsA(
          isA<GatewayException>()
              .having((e) => e.statusCode, 'statusCode', 404),
        ),
      );
      expect(provider.isPaired, isFalse);
    });

    test('rethrows GatewayException on network error', () async {
      final provider = await createProvider((request) async {
        throw Exception('Connection refused');
      });

      expect(
        () => provider.pair(testVin, testPin),
        throwsA(isA<GatewayException>()),
      );
      expect(provider.isPaired, isFalse);
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Auto-login (loadPersistedPairing)
  // ──────────────────────────────────────────────────────────────────────

  group('loadPersistedPairing()', () {
    test('restores isPaired when prefs contain token+VIN (Property 3)',
        () async {
      final provider = await createProvider(
        (request) async => http.Response('', 500),
        initialPrefs: {'vin': testVin, 'token': testToken},
      );

      expect(provider.isPaired, isFalse); // Not loaded yet.

      await provider.loadPersistedPairing();

      expect(provider.isPaired, isTrue);
      expect(provider.vin, testVin);
      expect(provider.token, testToken);
    });

    test('remains unpaired when prefs are empty', () async {
      final provider = await createProvider(
        (request) async => http.Response('', 500),
      );

      await provider.loadPersistedPairing();

      expect(provider.isPaired, isFalse);
    });

    test('remains unpaired when only VIN is stored (no token)', () async {
      final provider = await createProvider(
        (request) async => http.Response('', 500),
        initialPrefs: {'vin': testVin},
      );

      await provider.loadPersistedPairing();

      expect(provider.isPaired, isFalse);
    });

    test('notifies listeners', () async {
      final provider = await createProvider(
        (request) async => http.Response('', 500),
        initialPrefs: {'vin': testVin, 'token': testToken},
      );

      var notified = false;
      provider.addListener(() => notified = true);

      await provider.loadPersistedPairing();

      expect(notified, isTrue);
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Unpair
  // ──────────────────────────────────────────────────────────────────────

  group('unpair()', () {
    test('clears pairing state and prefs', () async {
      final provider = await createProvider(
        (request) async {
          return http.Response(
            jsonEncode({'token': testToken, 'vin': testVin}),
            200,
          );
        },
      );

      await provider.pair(testVin, testPin);
      expect(provider.isPaired, isTrue);

      await provider.unpair();

      expect(provider.isPaired, isFalse);
      expect(provider.vin, isNull);
      expect(provider.token, isNull);
      expect(provider.status, isNull);
      expect(provider.statusError, isNull);
      expect(provider.isCommandPending, isFalse);
      expect(provider.commandResult, isNull);
      expect(provider.commandError, isNull);

      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getString('vin'), isNull);
      expect(prefs.getString('token'), isNull);
    });

    test('notifies listeners', () async {
      final provider = await createProvider(
        (request) async {
          return http.Response(
            jsonEncode({'token': testToken, 'vin': testVin}),
            200,
          );
        },
      );

      await provider.pair(testVin, testPin);

      var notified = false;
      provider.addListener(() => notified = true);

      await provider.unpair();

      expect(notified, isTrue);
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Status polling
  // ──────────────────────────────────────────────────────────────────────

  group('startStatusPolling()', () {
    test('fetches status immediately on start', () async {
      var fetchCount = 0;
      final provider = await createProvider(
        (request) async {
          if (request.url.path.contains('/pair')) {
            return http.Response(
              jsonEncode({'token': testToken, 'vin': testVin}),
              200,
            );
          }
          fetchCount++;
          return http.Response(fullStatusJson(), 200);
        },
      );

      await provider.pair(testVin, testPin);

      // Use fakeAsync to control timers.
      await Future.microtask(() {});
      provider.startStatusPolling();

      // Let the immediate fetch complete.
      await Future.delayed(Duration.zero);
      await Future.delayed(Duration.zero);

      expect(fetchCount, greaterThanOrEqualTo(1));
      expect(provider.status, isNotNull);
      expect(provider.status!.vin, testVin);
      expect(provider.status!.isLocked, isTrue);
      expect(provider.statusError, isNull);

      provider.stopStatusPolling();
    });

    test('polls periodically every 5 seconds', () {
      fakeAsync((async) {
        var fetchCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        async.flushMicrotasks();

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              fetchCount++;
              return http.Response(fullStatusJson(), 200);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();
        expect(provider.isPaired, isTrue);

        provider.startStatusPolling();
        async.flushMicrotasks();

        // Immediate fetch = 1
        expect(fetchCount, 1);

        // After 5 seconds → 2
        async.elapse(const Duration(seconds: 5));
        async.flushMicrotasks();
        expect(fetchCount, 2);

        // After another 5 seconds → 3
        async.elapse(const Duration(seconds: 5));
        async.flushMicrotasks();
        expect(fetchCount, 3);

        provider.stopStatusPolling();

        // No more fetches after stopping.
        async.elapse(const Duration(seconds: 10));
        async.flushMicrotasks();
        expect(fetchCount, 3);
      });
    });

    test('is idempotent — calling twice does not double-poll', () {
      fakeAsync((async) {
        var fetchCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              fetchCount++;
              return http.Response(fullStatusJson(), 200);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.startStatusPolling();
        provider.startStatusPolling(); // Second call should be no-op
        async.flushMicrotasks();

        // Only 1 immediate fetch, not 2.
        expect(fetchCount, 1);

        async.elapse(const Duration(seconds: 5));
        async.flushMicrotasks();

        // Only 2 total, not 4.
        expect(fetchCount, 2);

        provider.stopStatusPolling();
      });
    });

    test(
        'preserves last status on poll failure (Property 5: Status Data Preservation)',
        () {
      fakeAsync((async) {
        var callCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              callCount++;
              if (callCount == 1) {
                // First call succeeds.
                return http.Response(fullStatusJson(), 200);
              }
              // Subsequent calls fail.
              throw Exception('Connection refused');
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.startStatusPolling();
        async.flushMicrotasks();

        // First fetch succeeded.
        expect(provider.status, isNotNull);
        expect(provider.status!.isLocked, isTrue);
        expect(provider.statusError, isNull);

        // Second fetch (after 5s) fails.
        async.elapse(const Duration(seconds: 5));
        async.flushMicrotasks();

        // Status preserved, error set.
        expect(provider.status, isNotNull);
        expect(provider.status!.isLocked, isTrue); // Still the old value.
        expect(provider.statusError, 'Connection lost');

        provider.stopStatusPolling();
      });
    });

    test('sets statusError on failure (Property 4: Error Visibility)', () {
      fakeAsync((async) {
        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              throw Exception('Connection refused');
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.startStatusPolling();
        async.flushMicrotasks();

        expect(provider.statusError, 'Connection lost');
        expect(provider.status, isNull); // No previous status to preserve.

        provider.stopStatusPolling();
      });
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Command feedback
  // ──────────────────────────────────────────────────────────────────────

  group('sendCommand()', () {
    test('sends lock command and receives success result', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-1';
        var pollCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                pollCount++;
                if (pollCount >= 2) {
                  // Second poll returns result.
                  return http.Response(
                    fullStatusJson(
                      commandId: commandId,
                      commandType: 'lock',
                      commandStatus: 'success',
                      commandResult: 'SUCCESS',
                    ),
                    200,
                  );
                }
                // First poll: still accepted.
                return http.Response(
                  fullStatusJson(
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'accepted',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();

        // Command pending immediately.
        expect(provider.isCommandPending, isTrue);
        expect(provider.commandResult, isNull);

        // First poll at 1s — still accepted.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.isCommandPending, isTrue);

        // Second poll at 2s — result available.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();

        expect(provider.isCommandPending, isFalse);
        expect(provider.commandResult, 'Locked successfully');
        expect(provider.commandError, isNull);
        expect(provider.status, isNotNull);
      });
    });

    test('sends unlock command and receives success result', () {
      fakeAsync((async) {
        const commandId = 'cmd-unlock-1';

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/unlock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                return http.Response(
                  fullStatusJson(
                    isLocked: false,
                    commandId: commandId,
                    commandType: 'unlock',
                    commandStatus: 'success',
                    commandResult: 'SUCCESS',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('unlock');
        async.flushMicrotasks();

        // First poll at 1s — result available immediately.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();

        expect(provider.isCommandPending, isFalse);
        expect(provider.commandResult, 'Unlocked successfully');
      });
    });

    test('displays rejection message for REJECTED_SPEED', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-speed';

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                return http.Response(
                  fullStatusJson(
                    speed: 80.0,
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'rejected',
                    commandResult: 'REJECTED_SPEED',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();

        expect(provider.isCommandPending, isFalse);
        expect(provider.commandResult, 'Rejected: vehicle speed too high');
      });
    });

    test('displays rejection message for REJECTED_DOOR_OPEN', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-door';

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                return http.Response(
                  fullStatusJson(
                    isDoorOpen: true,
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'rejected',
                    commandResult: 'REJECTED_DOOR_OPEN',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();

        expect(provider.isCommandPending, isFalse);
        expect(provider.commandResult, 'Rejected: door is open');
      });
    });

    test('times out after 10 polls without result (07-REQ-3.E1)', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-timeout';

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                // Always returns accepted — never completes.
                return http.Response(
                  fullStatusJson(
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'accepted',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();

        // Elapse 10 seconds (10 poll attempts).
        for (var i = 0; i < 10; i++) {
          async.elapse(const Duration(seconds: 1));
          async.flushMicrotasks();
        }

        expect(provider.isCommandPending, isFalse);
        expect(
          provider.commandResult,
          'Command timed out — check status manually.',
        );
      });
    });

    test('sets commandError on send failure (Property 4: Error Visibility)',
        () {
      fakeAsync((async) {
        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              throw Exception('Connection refused');
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();

        expect(provider.isCommandPending, isFalse);
        expect(provider.commandError, isNotNull);
        expect(provider.commandError, contains('Failed to send command'));
      });
    });

    test(
        'only matches correct command_id (Property 2: Command-Result Correlation)',
        () {
      fakeAsync((async) {
        const newCommandId = 'cmd-lock-new';
        const oldCommandId = 'cmd-lock-old';
        var pollCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': newCommandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                pollCount++;
                if (pollCount <= 2) {
                  // First polls: return old command result (different ID).
                  return http.Response(
                    fullStatusJson(
                      commandId: oldCommandId,
                      commandType: 'lock',
                      commandStatus: 'success',
                      commandResult: 'SUCCESS',
                    ),
                    200,
                  );
                }
                // Third poll: return matching new command result.
                return http.Response(
                  fullStatusJson(
                    commandId: newCommandId,
                    commandType: 'lock',
                    commandStatus: 'success',
                    commandResult: 'SUCCESS',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();

        // 1st poll: old command result — should not match.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.isCommandPending, isTrue);

        // 2nd poll: still old command — should not match.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.isCommandPending, isTrue);

        // 3rd poll: new command result — matches!
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.isCommandPending, isFalse);
        expect(provider.commandResult, 'Locked successfully');
      });
    });

    test('continues polling despite transient status fetch errors', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-retry';
        var pollCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                pollCount++;
                if (pollCount == 1) {
                  // First poll fails (transient error).
                  throw Exception('Timeout');
                }
                // Second poll succeeds with result.
                return http.Response(
                  fullStatusJson(
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'success',
                    commandResult: 'SUCCESS',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();

        // 1st poll at 1s: fails (transient error).
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.isCommandPending, isTrue);

        // 2nd poll at 2s: succeeds.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.isCommandPending, isFalse);
        expect(provider.commandResult, 'Locked successfully');
      });
    });

    test('clears previous command state before new command', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-2';

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                return http.Response(
                  fullStatusJson(
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'success',
                    commandResult: 'SUCCESS',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        // First command completes.
        provider.sendCommand('lock');
        async.flushMicrotasks();
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
        expect(provider.commandResult, 'Locked successfully');

        // Start new command — previous result should be cleared.
        provider.sendCommand('lock');
        async.flushMicrotasks();

        expect(provider.isCommandPending, isTrue);
        expect(provider.commandResult, isNull);
        expect(provider.commandError, isNull);

        // Let it complete.
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();
      });
    });

    test('handles unknown result code gracefully', () {
      fakeAsync((async) {
        const commandId = 'cmd-lock-unknown';

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              if (request.url.path.contains('/lock')) {
                return http.Response(
                  jsonEncode(
                      {'command_id': commandId, 'status': 'accepted'}),
                  202,
                );
              }
              if (request.url.path.contains('/status')) {
                return http.Response(
                  fullStatusJson(
                    commandId: commandId,
                    commandType: 'lock',
                    commandStatus: 'rejected',
                    commandResult: 'UNKNOWN_REASON',
                  ),
                  200,
                );
              }
              return http.Response('', 404);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.sendCommand('lock');
        async.flushMicrotasks();
        async.elapse(const Duration(seconds: 1));
        async.flushMicrotasks();

        expect(provider.commandResult, 'Command result: UNKNOWN_REASON');
      });
    });
  });

  // ──────────────────────────────────────────────────────────────────────
  // Dispose
  // ──────────────────────────────────────────────────────────────────────

  group('dispose()', () {
    test('cancels active timers', () {
      fakeAsync((async) {
        var fetchCount = 0;

        SharedPreferences.setMockInitialValues(
            {'vin': testVin, 'token': testToken});

        late VehicleProvider provider;

        SharedPreferences.getInstance().then((prefs) {
          final client = CloudGatewayClient(
            baseUrl: baseUrl,
            client: MockClient((request) async {
              fetchCount++;
              return http.Response(fullStatusJson(), 200);
            }),
          );
          provider = VehicleProvider(client: client, prefs: prefs);
          provider.loadPersistedPairing();
        });

        async.flushMicrotasks();

        provider.startStatusPolling();
        async.flushMicrotasks();

        final countBeforeDispose = fetchCount;
        provider.dispose();

        // Timers should be cancelled — no more fetches.
        async.elapse(const Duration(seconds: 20));
        async.flushMicrotasks();

        expect(fetchCount, countBeforeDispose);
      });
    });
  });
}
