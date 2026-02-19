package com.rhadp.parking.ui.adapter

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.rhadp.parking.data.ServiceException
import com.rhadp.parking.data.UpdateServiceClient
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import parking.common.Common.AdapterState

/**
 * ViewModel for the Adapter Status screen.
 *
 * Monitors adapter installation progress by streaming state updates
 * from UPDATE_SERVICE. Transitions to Ready when the adapter reaches
 * RUNNING state, enabling navigation to the Session Dashboard.
 *
 * Requirements: 06-REQ-2.1, 06-REQ-2.2, 06-REQ-2.3,
 *               06-REQ-2.E1, 06-REQ-2.E2
 */
class AdapterStatusViewModel(
    private val updateServiceClient: UpdateServiceClient,
) : ViewModel() {

    /**
     * UI states for the Adapter Status screen.
     */
    sealed class UiState {
        /** Adapter installation is in progress. */
        data class InProgress(val state: String) : UiState()

        /** Adapter is running and ready. Triggers navigation to Session Dashboard. */
        object Ready : UiState()

        /** An error occurred during installation. */
        data class Error(val message: String) : UiState()
    }

    private val _uiState = MutableStateFlow<UiState>(UiState.InProgress("INSTALLING"))

    /** Observable UI state for the Adapter Status screen. */
    val uiState: StateFlow<UiState> = _uiState.asStateFlow()

    /** The adapter ID being watched. Stored for retry support. */
    private var currentAdapterId: String? = null

    /** The current watch job, cancelled on retry or ViewModel clear. */
    private var watchJob: Job? = null

    /**
     * Starts watching adapter state transitions.
     *
     * Calls UPDATE_SERVICE.WatchAdapterStates() and processes state events.
     * When the adapter state reaches RUNNING, transitions to Ready (Property 5).
     * When the adapter state reaches ERROR, shows error with retry option.
     *
     * @param adapterId the adapter identifier to watch.
     */
    fun watchAdapter(adapterId: String) {
        currentAdapterId = adapterId
        _uiState.value = UiState.InProgress("INSTALLING")

        watchJob?.cancel()
        watchJob = viewModelScope.launch {
            try {
                // Stream adapter state events (06-REQ-2.1)
                updateServiceClient.watchAdapterStates().collect { event ->
                    // Only process events for our adapter
                    if (event.adapterId == adapterId) {
                        val stateName = formatAdapterState(event.newState)
                        Log.d(TAG, "Adapter $adapterId state: $stateName")

                        when (event.newState) {
                            // RUNNING → Ready (06-REQ-2.3, Property 5)
                            AdapterState.ADAPTER_STATE_RUNNING -> {
                                _uiState.value = UiState.Ready
                                return@collect
                            }
                            // ERROR → Error state (06-REQ-2.E1, Property 5)
                            AdapterState.ADAPTER_STATE_ERROR -> {
                                _uiState.value = UiState.Error(
                                    "Adapter installation failed"
                                )
                                return@collect
                            }
                            // In-progress states → update display (06-REQ-2.2)
                            else -> {
                                _uiState.value = UiState.InProgress(stateName)
                            }
                        }
                    }
                }
            } catch (e: ServiceException) {
                Log.e(TAG, "Watch stream failed for adapter $adapterId", e)
                _uiState.value = UiState.Error(e.message)
            } catch (e: Exception) {
                // UPDATE_SERVICE connection lost (06-REQ-2.E2)
                Log.e(TAG, "Connection lost watching adapter $adapterId", e)
                _uiState.value = UiState.Error(
                    "Connection to update service lost"
                )
            }
        }
    }

    /**
     * Retries adapter installation by re-starting the watch.
     *
     * Can be called after an error to attempt recovery.
     */
    fun retry() {
        currentAdapterId?.let { watchAdapter(it) }
    }

    /**
     * Formats an [AdapterState] proto enum to a user-friendly string.
     */
    private fun formatAdapterState(state: AdapterState): String {
        return when (state) {
            AdapterState.ADAPTER_STATE_DOWNLOADING -> "DOWNLOADING"
            AdapterState.ADAPTER_STATE_INSTALLING -> "INSTALLING"
            AdapterState.ADAPTER_STATE_RUNNING -> "RUNNING"
            AdapterState.ADAPTER_STATE_ERROR -> "ERROR"
            AdapterState.ADAPTER_STATE_STOPPED -> "STOPPED"
            else -> "UNKNOWN"
        }
    }

    companion object {
        private const val TAG = "AdapterStatusVM"
    }
}
