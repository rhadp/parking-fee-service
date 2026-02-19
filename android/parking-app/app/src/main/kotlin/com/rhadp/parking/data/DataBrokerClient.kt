package com.rhadp.parking.data

import android.util.Log
import com.rhadp.parking.model.Location
import io.grpc.ManagedChannel
import io.grpc.StatusException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kuksa.`val`.v2.Types.SignalID
import kuksa.`val`.v2.Val.GetValueRequest
import kuksa.`val`.v2.Val.SubscribeRequest
import kuksa.`val`.v2.VALGrpcKt

/**
 * Client for the Eclipse Kuksa DATA_BROKER.
 *
 * Reads vehicle signals via gRPC using the Kuksa val.v2 API.
 * Used for:
 * - Reading current vehicle location (lat/lon)
 * - Subscribing to Vehicle.Parking.SessionActive signal
 *
 * Requirements: 06-REQ-1.1, 06-REQ-3.1, 06-REQ-4.1
 */
class DataBrokerClient(
    private val channel: ManagedChannel,
) {
    private val stub = VALGrpcKt.VALCoroutineStub(channel)

    /**
     * Reads current vehicle location from DATA_BROKER.
     *
     * Reads Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude
     * signals and returns them as a [Location].
     *
     * @return the current location, or null if signals are not available.
     * @throws ServiceException if DATA_BROKER is unreachable or returns an error.
     */
    suspend fun getLocation(): Location? {
        try {
            val latResponse = stub.getValue(
                GetValueRequest.newBuilder()
                    .setSignalId(
                        SignalID.newBuilder()
                            .setPath("Vehicle.CurrentLocation.Latitude")
                            .build()
                    )
                    .build()
            )

            val lonResponse = stub.getValue(
                GetValueRequest.newBuilder()
                    .setSignalId(
                        SignalID.newBuilder()
                            .setPath("Vehicle.CurrentLocation.Longitude")
                            .build()
                    )
                    .build()
            )

            if (!latResponse.hasDataPoint() || !latResponse.dataPoint.hasValue()) return null
            if (!lonResponse.hasDataPoint() || !lonResponse.dataPoint.hasValue()) return null

            val lat = latResponse.dataPoint.value.double
            val lon = lonResponse.dataPoint.value.double

            return Location(latitude = lat, longitude = lon)
        } catch (e: StatusException) {
            Log.e(TAG, "Failed to read location from DATA_BROKER: ${e.status}", e)
            throw ServiceException("Unable to read vehicle location", e)
        }
    }

    /**
     * Subscribes to the Vehicle.Parking.SessionActive signal.
     *
     * Returns a [Flow] that emits `true` when a parking session starts
     * and `false` when it stops.
     *
     * @return a flow of session active states.
     */
    fun subscribeSessionActive(): Flow<Boolean> {
        val request = SubscribeRequest.newBuilder()
            .addSignalPaths("Vehicle.Parking.SessionActive")
            .build()

        return stub.subscribe(request)
            .map { response ->
                val entry = response.entriesMap["Vehicle.Parking.SessionActive"]
                entry?.value?.bool ?: false
            }
    }

    companion object {
        private const val TAG = "DataBrokerClient"
    }
}

/**
 * Exception indicating a service communication failure.
 *
 * Wraps gRPC or HTTP errors with a user-friendly message.
 * Requirements: 06-REQ-4.E1
 */
class ServiceException(
    override val message: String,
    override val cause: Throwable? = null,
) : Exception(message, cause)
