package com.rhadp.parking.ui.session

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.rhadp.parking.data.DataBrokerClient
import com.rhadp.parking.data.ParkingAdapterClient
import com.rhadp.parking.data.ServiceConfig
import com.rhadp.parking.data.ServiceException
import com.rhadp.parking.model.SessionInfo
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/**
 * ViewModel for the Session Dashboard screen.
 *
 * Monitors the parking session lifecycle:
 * 1. Subscribes to SessionActive signal from DATA_BROKER
 * 2. When session starts, polls PARKING_OPERATOR_ADAPTOR for status every 5s
 * 3. When session stops, displays final summary
 *
 * Requirements: 06-REQ-3.1, 06-REQ-3.2, 06-REQ-3.3, 06-REQ-3.4,
 *               06-REQ-3.E1
 */
class SessionDashboardViewModel(
    private val dataBrokerClient: DataBrokerClient,
    private val adapterClient: ParkingAdapterClient,
    private val pollIntervalMs: Long = ServiceConfig.SESSION_POLL_INTERVAL_MS,
) : ViewModel() {

    /**
     * UI states for the Session Dashboard screen.
     */
    sealed class UiState {
        /** Adapter is running, waiting for session to start. (06-REQ-3.4) */
        object WaitingForSession : UiState()

        /** Session is active, showing live fee updates. (06-REQ-3.2) */
        data class SessionActive(
            val sessionId: String,
            val startTime: Long,
            val currentFee: Double,
            val currency: String,
            val zoneName: String,
            val connectionLost: Boolean = false,
        ) : UiState()

        /** Session has completed, showing summary. (06-REQ-3.3) */
        data class SessionCompleted(
            val totalFee: Double,
            val durationSeconds: Long,
            val currency: String,
        ) : UiState()

        /** An error occurred. */
        data class Error(val message: String) : UiState()
    }

    private val _uiState = MutableStateFlow<UiState>(UiState.WaitingForSession)

    /** Observable UI state for the Session Dashboard screen. */
    val uiState: StateFlow<UiState> = _uiState.asStateFlow()

    /** The monitoring job for SessionActive subscription. */
    private var monitorJob: Job? = null

    /** The polling job for GetStatus calls. */
    private var pollJob: Job? = null

    /** Last known session info for connection-lost recovery. */
    private var lastKnownSession: SessionInfo? = null

    /**
     * Starts monitoring the parking session lifecycle.
     *
     * Subscribes to DATA_BROKER's SessionActive signal (06-REQ-3.1).
     * When SessionActive becomes true, starts polling GetStatus.
     * When SessionActive becomes false, fetches final status and shows summary.
     *
     * State transitions (Property 3: Session State Consistency):
     * - WaitingForSession → SessionActive (SessionActive = true)
     * - SessionActive → SessionCompleted (SessionActive = false)
     * - Any → Error (subscription failure)
     */
    fun startMonitoring() {
        monitorJob?.cancel()
        pollJob?.cancel()
        _uiState.value = UiState.WaitingForSession

        monitorJob = viewModelScope.launch {
            try {
                // Subscribe to SessionActive signal (06-REQ-3.1)
                dataBrokerClient.subscribeSessionActive().collect { isActive ->
                    Log.d(TAG, "SessionActive = $isActive")

                    if (isActive) {
                        // Session started — begin polling (06-REQ-3.2)
                        startPolling()
                    } else {
                        // Session stopped — show summary (06-REQ-3.3)
                        stopPolling()
                        showSessionSummary()
                    }
                }
            } catch (e: ServiceException) {
                Log.e(TAG, "SessionActive subscription failed", e)
                _uiState.value = UiState.Error(e.message)
            } catch (e: Exception) {
                Log.e(TAG, "Unexpected error in session monitoring", e)
                _uiState.value = UiState.Error("Failed to monitor parking session")
            }
        }
    }

    /**
     * Starts polling GetStatus every [pollIntervalMs] milliseconds.
     *
     * Continues until cancelled or until a connection error occurs.
     * On connection loss, shows last known info with indicator (06-REQ-3.E1).
     */
    private fun startPolling() {
        pollJob?.cancel()
        pollJob = viewModelScope.launch {
            while (isActive) {
                try {
                    val status = adapterClient.getStatus("")
                    lastKnownSession = status

                    _uiState.value = UiState.SessionActive(
                        sessionId = status.sessionId,
                        startTime = status.startTime,
                        currentFee = status.currentFee,
                        currency = "EUR",
                        zoneName = "",
                        connectionLost = false,
                    )
                } catch (e: ServiceException) {
                    // Connection lost — show last known info (06-REQ-3.E1)
                    Log.w(TAG, "Adapter unreachable during polling", e)
                    val lastKnown = lastKnownSession
                    if (lastKnown != null) {
                        _uiState.value = UiState.SessionActive(
                            sessionId = lastKnown.sessionId,
                            startTime = lastKnown.startTime,
                            currentFee = lastKnown.currentFee,
                            currency = "EUR",
                            zoneName = "",
                            connectionLost = true,
                        )
                    } else {
                        _uiState.value = UiState.Error(e.message)
                    }
                }
                delay(pollIntervalMs)
            }
        }
    }

    /**
     * Stops the polling job.
     */
    private fun stopPolling() {
        pollJob?.cancel()
        pollJob = null
    }

    /**
     * Fetches final session status and displays summary.
     *
     * Called when SessionActive transitions to false (06-REQ-3.3).
     * Calls GetStatus once to get the final fee and duration.
     */
    private suspend fun showSessionSummary() {
        try {
            val finalStatus = adapterClient.getStatus("")
            lastKnownSession = finalStatus

            val durationSeconds = if (finalStatus.startTime > 0) {
                (System.currentTimeMillis() / 1000) - finalStatus.startTime
            } else {
                0L
            }

            _uiState.value = UiState.SessionCompleted(
                totalFee = finalStatus.currentFee,
                durationSeconds = durationSeconds,
                currency = "EUR",
            )
        } catch (e: ServiceException) {
            Log.w(TAG, "Failed to get final session status", e)
            // Use last known session if available
            val lastKnown = lastKnownSession
            if (lastKnown != null) {
                val durationSeconds = if (lastKnown.startTime > 0) {
                    (System.currentTimeMillis() / 1000) - lastKnown.startTime
                } else {
                    0L
                }
                _uiState.value = UiState.SessionCompleted(
                    totalFee = lastKnown.currentFee,
                    durationSeconds = durationSeconds,
                    currency = "EUR",
                )
            } else {
                _uiState.value = UiState.Error(e.message)
            }
        }
    }

    companion object {
        private const val TAG = "SessionDashboardVM"
    }
}
