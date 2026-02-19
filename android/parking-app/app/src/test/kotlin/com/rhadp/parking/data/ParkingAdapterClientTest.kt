package com.rhadp.parking.data

import io.grpc.ManagedChannel
import io.grpc.Status
import io.grpc.inprocess.InProcessChannelBuilder
import io.grpc.inprocess.InProcessServerBuilder
import io.grpc.testing.GrpcCleanupRule
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import parking.services.adapter.ParkingAdapterGrpcKt
import parking.services.adapter.ParkingAdapterOuterClass.GetStatusRequest
import parking.services.adapter.ParkingAdapterOuterClass.GetStatusResponse

/**
 * Unit tests for [ParkingAdapterClient].
 *
 * Uses grpc-testing InProcessServer to verify correct gRPC call
 * construction and response parsing without a real PARKING_OPERATOR_ADAPTOR.
 *
 * Requirements: 06-REQ-7.2, 06-REQ-7.3
 */
class ParkingAdapterClientTest {

    @get:Rule
    val grpcCleanup = GrpcCleanupRule()

    private lateinit var channel: ManagedChannel
    private lateinit var client: ParkingAdapterClient

    private lateinit var fakeService: FakeParkingAdapterService

    @Before
    fun setUp() {
        fakeService = FakeParkingAdapterService()

        val serverName = InProcessServerBuilder.generateName()
        grpcCleanup.register(
            InProcessServerBuilder.forName(serverName)
                .directExecutor()
                .addService(fakeService)
                .build()
                .start()
        )
        channel = grpcCleanup.register(
            InProcessChannelBuilder.forName(serverName)
                .directExecutor()
                .build()
        )
        client = ParkingAdapterClient(channel)
    }

    @Test
    fun `getStatus sends correct session ID`() = runTest {
        fakeService.statusResponse = GetStatusResponse.newBuilder()
            .setSessionId("session-42")
            .setActive(true)
            .setStartTime(1700000000L)
            .setCurrentFee(0.15)
            .build()

        client.getStatus("session-42")

        assertEquals(1, fakeService.getStatusRequests.size)
        assertEquals("session-42", fakeService.getStatusRequests[0].sessionId)
    }

    @Test
    fun `getStatus returns correct SessionInfo for active session`() = runTest {
        fakeService.statusResponse = GetStatusResponse.newBuilder()
            .setSessionId("session-99")
            .setActive(true)
            .setStartTime(1700000000L)
            .setCurrentFee(0.25)
            .build()

        val info = client.getStatus("session-99")

        assertEquals("session-99", info.sessionId)
        assertTrue(info.active)
        assertEquals(1700000000L, info.startTime)
        assertEquals(0.25, info.currentFee, 0.001)
    }

    @Test
    fun `getStatus returns correct SessionInfo for completed session`() = runTest {
        fakeService.statusResponse = GetStatusResponse.newBuilder()
            .setSessionId("session-77")
            .setActive(false)
            .setStartTime(1700000000L)
            .setCurrentFee(1.50)
            .build()

        val info = client.getStatus("session-77")

        assertEquals("session-77", info.sessionId)
        assertEquals(false, info.active)
        assertEquals(1.50, info.currentFee, 0.001)
    }

    @Test
    fun `getStatus throws ServiceException on gRPC error`() = runTest {
        fakeService.getStatusError = Status.UNAVAILABLE
            .withDescription("Adapter offline")
            .asException()

        try {
            client.getStatus("session-1")
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertEquals("Unable to get parking session status", e.message)
        }
    }

    @Test
    fun `getStatus throws ServiceException on NOT_FOUND`() = runTest {
        fakeService.getStatusError = Status.NOT_FOUND
            .withDescription("Session not found")
            .asException()

        try {
            client.getStatus("nonexistent")
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertEquals("Unable to get parking session status", e.message)
        }
    }

    @Test
    fun `getStatusRaw returns raw proto response`() = runTest {
        val expected = GetStatusResponse.newBuilder()
            .setSessionId("session-raw")
            .setActive(true)
            .setStartTime(1700000000L)
            .setCurrentFee(0.75)
            .build()
        fakeService.statusResponse = expected

        val response = client.getStatusRaw("session-raw")

        assertEquals(expected, response)
    }

    /**
     * Fake ParkingAdapter service for testing.
     */
    private class FakeParkingAdapterService :
        ParkingAdapterGrpcKt.ParkingAdapterCoroutineImplBase() {

        val getStatusRequests = mutableListOf<GetStatusRequest>()
        var statusResponse: GetStatusResponse = GetStatusResponse.getDefaultInstance()
        var getStatusError: Exception? = null

        override suspend fun getStatus(request: GetStatusRequest): GetStatusResponse {
            getStatusRequests.add(request)
            getStatusError?.let { throw it }
            return statusResponse
        }
    }
}
