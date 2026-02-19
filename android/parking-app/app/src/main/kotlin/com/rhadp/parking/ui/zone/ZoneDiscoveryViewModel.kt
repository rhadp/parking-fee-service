package com.rhadp.parking.ui.zone

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.rhadp.parking.data.DataBrokerClient
import com.rhadp.parking.data.ParkingFeeServiceClient
import com.rhadp.parking.data.ServiceException
import com.rhadp.parking.data.UpdateServiceClient
import com.rhadp.parking.model.ZoneMatch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * ViewModel for the Zone Discovery screen.
 *
 * Orchestrates the zone discovery flow:
 * 1. Read vehicle location from DATA_BROKER
 * 2. Query PARKING_FEE_SERVICE for nearby zones
 * 3. On zone selection, fetch adapter metadata and install via UPDATE_SERVICE
 *
 * Requirements: 06-REQ-1.1, 06-REQ-1.2, 06-REQ-1.3, 06-REQ-1.4,
 *               06-REQ-1.E1, 06-REQ-1.E2, 06-REQ-1.E3
 */
class ZoneDiscoveryViewModel(
    private val dataBrokerClient: DataBrokerClient,
    private val pfsClient: ParkingFeeServiceClient,
    private val updateServiceClient: UpdateServiceClient,
) : ViewModel() {

    /**
     * UI states for the Zone Discovery screen.
     */
    sealed class UiState {
        /** Loading location and zones. */
        object Loading : UiState()

        /** Zones found near the vehicle. */
        data class ZonesFound(val zones: List<ZoneMatch>) : UiState()

        /** No zones found near the vehicle. */
        object NoZones : UiState()

        /** An error occurred. */
        data class Error(val message: String) : UiState()

        /** Adapter installation has been requested for the selected zone. */
        data class Installing(val zoneId: String, val adapterId: String) : UiState()
    }

    private val _uiState = MutableStateFlow<UiState>(UiState.Loading)

    /** Observable UI state for the Zone Discovery screen. */
    val uiState: StateFlow<UiState> = _uiState.asStateFlow()

    /**
     * Loads nearby parking zones.
     *
     * Reads the vehicle location from DATA_BROKER, then queries
     * PARKING_FEE_SERVICE for zones near that location.
     *
     * State transitions:
     * - Loading → ZonesFound (zones returned)
     * - Loading → NoZones (empty list returned)
     * - Loading → Error (DATA_BROKER or PFS unreachable)
     */
    fun loadZones() {
        _uiState.value = UiState.Loading
        viewModelScope.launch {
            try {
                // Step 1: Read location from DATA_BROKER (06-REQ-1.1)
                val location = dataBrokerClient.getLocation()
                if (location == null) {
                    _uiState.value = UiState.Error("Vehicle location not available")
                    return@launch
                }

                // Step 2: Query PFS for nearby zones (06-REQ-1.2)
                val zones = pfsClient.lookupZones(location.latitude, location.longitude)

                // Step 3: Update state based on result (06-REQ-1.E1)
                _uiState.value = if (zones.isEmpty()) {
                    UiState.NoZones
                } else {
                    UiState.ZonesFound(zones)
                }
            } catch (e: ServiceException) {
                Log.e(TAG, "Failed to load zones", e)
                _uiState.value = UiState.Error(e.message)
            } catch (e: Exception) {
                Log.e(TAG, "Unexpected error loading zones", e)
                _uiState.value = UiState.Error("Failed to load parking zones")
            }
        }
    }

    /**
     * Selects a zone and starts adapter installation.
     *
     * Retrieves adapter metadata from PARKING_FEE_SERVICE for the
     * selected zone, then calls UPDATE_SERVICE to install the adapter.
     * The imageRef and checksum from PFS are passed through unmodified
     * (Property 2: Adapter Install Trigger).
     *
     * State transitions:
     * - ZonesFound → Installing (install started)
     * - ZonesFound → Error (PFS or UPDATE_SERVICE unreachable)
     *
     * @param zone the zone selected by the user.
     */
    fun selectZone(zone: ZoneMatch) {
        viewModelScope.launch {
            try {
                // Step 1: Get adapter metadata from PFS (06-REQ-1.4)
                val metadata = pfsClient.getZoneAdapter(zone.zoneId)

                // Step 2: Install adapter via UPDATE_SERVICE (06-REQ-1.4)
                // imageRef and checksum passed through unmodified (Property 2)
                val response = updateServiceClient.installAdapter(
                    metadata.imageRef,
                    metadata.checksum,
                )

                _uiState.value = UiState.Installing(
                    zoneId = zone.zoneId,
                    adapterId = response.adapterId,
                )
            } catch (e: ServiceException) {
                Log.e(TAG, "Failed to install adapter for zone ${zone.zoneId}", e)
                _uiState.value = UiState.Error(e.message)
            } catch (e: Exception) {
                Log.e(TAG, "Unexpected error during zone selection", e)
                _uiState.value = UiState.Error("Failed to install parking adapter")
            }
        }
    }

    companion object {
        private const val TAG = "ZoneDiscoveryVM"
    }
}
