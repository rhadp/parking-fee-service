/// State management for vehicle pairing, status polling, and command feedback.
///
/// Uses [ChangeNotifier] with the Provider package for reactive UI updates.
/// Persists pairing credentials via [SharedPreferences].
library;

import 'dart:async';
import 'dart:developer' as developer;

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/models.dart';
import '../services/cloud_gateway_client.dart';

/// Manages vehicle state: pairing, status polling, and lock/unlock commands.
///
/// Exposes reactive state for the UI via [ChangeNotifier]. Handles:
/// - Pairing/unpairing with VIN + PIN (persisted to [SharedPreferences]).
/// - Periodic status polling every 5 seconds.
/// - Lock/unlock command dispatch with 1-second result polling (up to 10s).
/// - Error state management preserving last known status on failure.
class VehicleProvider extends ChangeNotifier {
  final CloudGatewayClient _client;
  final SharedPreferences _prefs;

  // ── Pairing state ──────────────────────────────────────────────────────

  String? _vin;
  String? _token;

  /// Whether the app has an active pairing (token + VIN present).
  bool get isPaired => _token != null && _vin != null;

  /// The paired vehicle's VIN, or null if not paired.
  String? get vin => _vin;

  /// The bearer token, or null if not paired.
  String? get token => _token;

  // ── Vehicle status ─────────────────────────────────────────────────────

  VehicleStatus? _status;

  /// The most recently fetched vehicle status, or null if never fetched.
  VehicleStatus? get status => _status;

  bool _isStatusLoading = false;

  /// Whether a status fetch is currently in progress.
  bool get isStatusLoading => _isStatusLoading;

  String? _statusError;

  /// Error message from the last failed status poll, or null.
  String? get statusError => _statusError;

  // ── Command state ──────────────────────────────────────────────────────

  bool _isCommandPending = false;

  /// Whether a lock/unlock command is in flight (waiting for result).
  bool get isCommandPending => _isCommandPending;

  String? _commandResult;

  /// User-friendly result message after a command completes.
  String? get commandResult => _commandResult;

  String? _commandError;

  /// Error message if a command failed to send.
  String? get commandError => _commandError;

  // ── Timers ─────────────────────────────────────────────────────────────

  Timer? _statusTimer;
  Timer? _commandPollTimer;

  /// Creates a [VehicleProvider].
  ///
  /// Requires a [CloudGatewayClient] for REST calls and [SharedPreferences]
  /// for token persistence.
  VehicleProvider({
    required CloudGatewayClient client,
    required SharedPreferences prefs,
  })  : _client = client,
        _prefs = prefs;

  // ── Pairing ────────────────────────────────────────────────────────────

  /// Load persisted pairing credentials on app startup.
  ///
  /// If a token and VIN are stored in [SharedPreferences], restores them
  /// so the user does not need to re-pair.
  Future<void> loadPersistedPairing() async {
    _vin = _prefs.getString('vin');
    _token = _prefs.getString('token');
    developer.log(
      'Loaded persisted pairing: isPaired=$isPaired, vin=$_vin',
      name: 'VehicleProvider',
    );
    notifyListeners();
  }

  /// Pair with a vehicle using VIN and PIN.
  ///
  /// On success, persists the token and VIN to [SharedPreferences] and
  /// sets [isPaired] to true. On failure, rethrows the exception for the
  /// UI to handle.
  Future<void> pair(String vin, String pin) async {
    developer.log('Pairing with VIN=$vin', name: 'VehicleProvider');
    final response = await _client.pair(vin, pin);
    _vin = response.vin;
    _token = response.token;
    await _prefs.setString('vin', _vin!);
    await _prefs.setString('token', _token!);
    developer.log('Paired successfully: vin=$_vin', name: 'VehicleProvider');
    notifyListeners();
  }

  /// Clear the current pairing.
  ///
  /// Stops any active polling, clears in-memory state, and removes
  /// persisted credentials from [SharedPreferences].
  Future<void> unpair() async {
    developer.log('Unpairing', name: 'VehicleProvider');
    _stopStatusPolling();
    _stopCommandPolling();
    _vin = null;
    _token = null;
    _status = null;
    _statusError = null;
    _isStatusLoading = false;
    _isCommandPending = false;
    _commandResult = null;
    _commandError = null;
    await _prefs.remove('vin');
    await _prefs.remove('token');
    notifyListeners();
  }

  // ── Status polling ─────────────────────────────────────────────────────

  /// Start polling vehicle status every 5 seconds.
  ///
  /// Performs an immediate first fetch, then sets up a periodic timer.
  /// If polling is already active, this is a no-op.
  void startStatusPolling() {
    if (_statusTimer != null) return; // Already polling
    developer.log('Starting status polling', name: 'VehicleProvider');
    _fetchStatus(); // Immediate first fetch
    _statusTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => _fetchStatus(),
    );
  }

  /// Stop status polling.
  void stopStatusPolling() {
    _stopStatusPolling();
  }

  void _stopStatusPolling() {
    _statusTimer?.cancel();
    _statusTimer = null;
  }

  void _stopCommandPolling() {
    _commandPollTimer?.cancel();
    _commandPollTimer = null;
  }

  /// Fetch the current vehicle status from the gateway.
  ///
  /// On success, updates [status] and clears any previous [statusError].
  /// On failure, sets [statusError] but preserves the last known [status]
  /// (Property 5: Status Data Preservation).
  Future<void> _fetchStatus() async {
    if (!isPaired) return;

    _isStatusLoading = true;
    notifyListeners();

    try {
      _status = await _client.getStatus(_vin!, _token!);
      _statusError = null;
      developer.log(
        'Status updated: locked=${_status?.isLocked}',
        name: 'VehicleProvider',
      );
    } catch (e) {
      // Preserve last known status on failure (Property 5).
      _statusError = 'Connection lost';
      developer.log(
        'Status fetch failed: $e',
        name: 'VehicleProvider',
      );
    }

    _isStatusLoading = false;
    notifyListeners();
  }

  // ── Commands ───────────────────────────────────────────────────────────

  /// Send a lock or unlock command and poll for the result.
  ///
  /// Sets [isCommandPending] to true during the operation. On completion,
  /// sets [commandResult] with a user-friendly message. On failure, sets
  /// [commandError].
  ///
  /// The [type] parameter must be `'lock'` or `'unlock'`.
  Future<void> sendCommand(String type) async {
    if (!isPaired) return;

    _isCommandPending = true;
    _commandResult = null;
    _commandError = null;
    notifyListeners();

    developer.log('Sending $type command', name: 'VehicleProvider');

    try {
      final cmdResponse = type == 'lock'
          ? await _client.lock(_vin!, _token!)
          : await _client.unlock(_vin!, _token!);

      developer.log(
        'Command accepted: id=${cmdResponse.commandId}',
        name: 'VehicleProvider',
      );

      // Poll for result every 1s, up to 10s.
      await _pollForCommandResult(cmdResponse.commandId);
    } catch (e) {
      _commandError = 'Failed to send command: $e';
      _isCommandPending = false;
      developer.log(
        'Command failed: $e',
        name: 'VehicleProvider',
      );
      notifyListeners();
    }
  }

  /// Poll the vehicle status every 1 second looking for a matching command
  /// result. Times out after 10 attempts.
  ///
  /// Property 2: Command-Result Correlation — only accepts a result when
  /// `last_command.command_id` matches the sent command's ID.
  Future<void> _pollForCommandResult(String commandId) async {
    final completer = Completer<void>();
    var attempts = 0;

    _commandPollTimer = Timer.periodic(
      const Duration(seconds: 1),
      (timer) async {
        attempts++;
        developer.log(
          'Command poll attempt $attempts for $commandId',
          name: 'VehicleProvider',
        );

        try {
          final polledStatus = await _client.getStatus(_vin!, _token!);

          // Property 2: Only match on the exact command_id.
          if (polledStatus.lastCommand?.commandId == commandId &&
              polledStatus.lastCommand?.status != 'accepted') {
            timer.cancel();
            _commandPollTimer = null;
            _commandResult = _formatResult(polledStatus.lastCommand!);
            _isCommandPending = false;
            _status = polledStatus;
            developer.log(
              'Command result: $_commandResult',
              name: 'VehicleProvider',
            );
            notifyListeners();
            if (!completer.isCompleted) completer.complete();
            return;
          }
        } catch (e) {
          developer.log(
            'Command poll error: $e',
            name: 'VehicleProvider',
          );
          // Continue polling — transient errors should not abort feedback.
        }

        if (attempts >= 10) {
          timer.cancel();
          _commandPollTimer = null;
          _commandResult = 'Command timed out — check status manually.';
          _isCommandPending = false;
          developer.log(
            'Command poll timed out after $attempts attempts',
            name: 'VehicleProvider',
          );
          notifyListeners();
          if (!completer.isCompleted) completer.complete();
        }
      },
    );

    return completer.future;
  }

  /// Format a [CommandInfo] result into a user-friendly message.
  ///
  /// Maps result codes to human-readable strings per 07-REQ-3.4.
  String _formatResult(CommandInfo cmd) {
    switch (cmd.result) {
      case 'SUCCESS':
        return cmd.type == 'lock'
            ? 'Locked successfully'
            : 'Unlocked successfully';
      case 'REJECTED_SPEED':
        return 'Rejected: vehicle speed too high';
      case 'REJECTED_DOOR_OPEN':
        return 'Rejected: door is open';
      default:
        return 'Command result: ${cmd.result}';
    }
  }

  // ── Cleanup ────────────────────────────────────────────────────────────

  @override
  void dispose() {
    _stopStatusPolling();
    _stopCommandPolling();
    super.dispose();
  }
}
