package com.rhadp.parking.data

/**
 * Default service addresses for the PARKING_APP.
 *
 * `10.0.2.2` is the Android emulator alias for the host machine's localhost.
 * When running on a real device or Cuttlefish, these must be reconfigured
 * to point to the actual service hosts.
 *
 * Requirements: 06-REQ-5.1, 06-REQ-5.2
 */
object ServiceConfig {
    /** DATA_BROKER (Kuksa) gRPC address. */
    const val DATABROKER_HOST = "10.0.2.2"
    const val DATABROKER_PORT = 55555

    /** UPDATE_SERVICE gRPC address. */
    const val UPDATE_SERVICE_HOST = "10.0.2.2"
    const val UPDATE_SERVICE_PORT = 50053

    /** PARKING_OPERATOR_ADAPTOR gRPC address. */
    const val ADAPTER_HOST = "10.0.2.2"
    const val ADAPTER_PORT = 50054

    /** PARKING_FEE_SERVICE REST base URL. */
    const val PFS_BASE_URL = "http://10.0.2.2:8080"

    /** Polling interval for GetStatus calls during active sessions. */
    const val SESSION_POLL_INTERVAL_MS = 5000L
}
