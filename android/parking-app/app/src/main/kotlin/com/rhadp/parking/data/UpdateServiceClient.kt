package com.rhadp.parking.data

import android.util.Log
import io.grpc.ManagedChannel
import io.grpc.StatusException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import parking.common.Common.AdapterState
import parking.services.update.UpdateServiceGrpcKt
import parking.services.update.UpdateServiceOuterClass.AdapterStateEvent
import parking.services.update.UpdateServiceOuterClass.InstallAdapterRequest
import parking.services.update.UpdateServiceOuterClass.InstallAdapterResponse
import parking.services.update.UpdateServiceOuterClass.WatchAdapterStatesRequest

/**
 * Client for the UPDATE_SERVICE gRPC API.
 *
 * Manages adapter container lifecycle: installation and state monitoring.
 * Uses grpc-kotlin coroutine stubs for async communication.
 *
 * Requirements: 06-REQ-1.4, 06-REQ-2.1, 06-REQ-4.1
 */
class UpdateServiceClient(
    private val channel: ManagedChannel,
) {
    private val stub = UpdateServiceGrpcKt.UpdateServiceCoroutineStub(channel)

    /**
     * Installs an adapter container.
     *
     * Calls `UpdateService.InstallAdapter` with the given image reference
     * and checksum. Returns the response containing the job and adapter IDs.
     *
     * @param imageRef the container image reference (e.g. "gcr.io/project/adapter:v1").
     * @param checksum the image checksum for verification.
     * @return the install response with job ID and initial state.
     * @throws ServiceException if the service is unreachable or returns an error.
     */
    suspend fun installAdapter(
        imageRef: String,
        checksum: String,
    ): InstallAdapterResponse {
        try {
            return stub.installAdapter(
                InstallAdapterRequest.newBuilder()
                    .setImageRef(imageRef)
                    .setChecksum(checksum)
                    .build()
            )
        } catch (e: StatusException) {
            Log.e(TAG, "Failed to install adapter: ${e.status}", e)
            throw ServiceException("Unable to install parking adapter", e)
        }
    }

    /**
     * Streams adapter state changes.
     *
     * Calls `UpdateService.WatchAdapterStates` and returns a [Flow] of
     * [AdapterStateUpdate] events representing state transitions.
     *
     * @return a flow of adapter state updates.
     */
    fun watchAdapterStates(): Flow<AdapterStateUpdate> {
        return stub.watchAdapterStates(
            WatchAdapterStatesRequest.getDefaultInstance()
        ).map { event ->
            AdapterStateUpdate(
                adapterId = event.adapterId,
                oldState = event.oldState,
                newState = event.newState,
                timestamp = event.timestamp,
            )
        }
    }

    companion object {
        private const val TAG = "UpdateServiceClient"
    }
}

/**
 * Simplified adapter state update event.
 *
 * Maps from the proto [AdapterStateEvent] to a Kotlin data class
 * for easier consumption in ViewModels.
 */
data class AdapterStateUpdate(
    val adapterId: String,
    val oldState: AdapterState,
    val newState: AdapterState,
    val timestamp: Long,
)
