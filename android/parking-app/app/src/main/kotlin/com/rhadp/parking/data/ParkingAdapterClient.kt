package com.rhadp.parking.data

import android.util.Log
import com.rhadp.parking.model.SessionInfo
import io.grpc.ManagedChannel
import io.grpc.StatusException
import parking.services.adapter.ParkingAdapterGrpcKt
import parking.services.adapter.ParkingAdapterOuterClass.GetStatusRequest
import parking.services.adapter.ParkingAdapterOuterClass.GetStatusResponse

/**
 * Client for the PARKING_OPERATOR_ADAPTOR gRPC API.
 *
 * Retrieves parking session status from the operator adapter running
 * in the QM partition. Used during active parking sessions to poll
 * for fee updates.
 *
 * Requirements: 06-REQ-3.2, 06-REQ-4.1
 */
class ParkingAdapterClient(
    private val channel: ManagedChannel,
) {
    private val stub = ParkingAdapterGrpcKt.ParkingAdapterCoroutineStub(channel)

    /**
     * Gets the current status of a parking session.
     *
     * Calls `ParkingAdapter.GetStatus` with the given session ID and
     * returns the response as a [SessionInfo] data class.
     *
     * @param sessionId the parking session identifier.
     * @return the current session status.
     * @throws ServiceException if the adapter is unreachable or returns an error.
     */
    suspend fun getStatus(sessionId: String): SessionInfo {
        try {
            val response = stub.getStatus(
                GetStatusRequest.newBuilder()
                    .setSessionId(sessionId)
                    .build()
            )
            return SessionInfo(
                sessionId = response.sessionId,
                active = response.active,
                startTime = response.startTime,
                currentFee = response.currentFee,
            )
        } catch (e: StatusException) {
            Log.e(TAG, "Failed to get session status: ${e.status}", e)
            throw ServiceException("Unable to get parking session status", e)
        }
    }

    /**
     * Gets the raw gRPC response for a parking session status.
     *
     * This is useful when the caller needs access to the full proto
     * response object.
     *
     * @param sessionId the parking session identifier.
     * @return the raw GetStatusResponse proto.
     * @throws ServiceException if the adapter is unreachable or returns an error.
     */
    suspend fun getStatusRaw(sessionId: String): GetStatusResponse {
        try {
            return stub.getStatus(
                GetStatusRequest.newBuilder()
                    .setSessionId(sessionId)
                    .build()
            )
        } catch (e: StatusException) {
            Log.e(TAG, "Failed to get session status: ${e.status}", e)
            throw ServiceException("Unable to get parking session status", e)
        }
    }

    companion object {
        private const val TAG = "ParkingAdapterClient"
    }
}
