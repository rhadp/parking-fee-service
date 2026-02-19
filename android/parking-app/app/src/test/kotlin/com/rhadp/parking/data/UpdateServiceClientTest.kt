package com.rhadp.parking.data

import io.grpc.ManagedChannel
import io.grpc.Status
import io.grpc.inprocess.InProcessChannelBuilder
import io.grpc.inprocess.InProcessServerBuilder
import io.grpc.testing.GrpcCleanupRule
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import parking.common.Common.AdapterState
import parking.services.update.UpdateServiceGrpcKt
import parking.services.update.UpdateServiceOuterClass.AdapterStateEvent
import parking.services.update.UpdateServiceOuterClass.InstallAdapterRequest
import parking.services.update.UpdateServiceOuterClass.InstallAdapterResponse
import parking.services.update.UpdateServiceOuterClass.WatchAdapterStatesRequest

/**
 * Unit tests for [UpdateServiceClient].
 *
 * Uses grpc-testing InProcessServer to verify correct gRPC call
 * construction and response handling without a real UPDATE_SERVICE.
 *
 * Tests Property 2 (Adapter Install Trigger): verifies that imageRef and
 * checksum are passed through unmodified.
 *
 * Requirements: 06-REQ-7.2, 06-REQ-7.3
 */
class UpdateServiceClientTest {

    @get:Rule
    val grpcCleanup = GrpcCleanupRule()

    private lateinit var channel: ManagedChannel
    private lateinit var client: UpdateServiceClient

    private lateinit var fakeService: FakeUpdateService

    @Before
    fun setUp() {
        fakeService = FakeUpdateService()

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
        client = UpdateServiceClient(channel)
    }

    // --- installAdapter ---

    @Test
    fun `installAdapter sends correct imageRef and checksum`() = runTest {
        fakeService.installResponse = InstallAdapterResponse.newBuilder()
            .setJobId("job-1")
            .setAdapterId("adapter-1")
            .setState(AdapterState.ADAPTER_STATE_DOWNLOADING)
            .build()

        client.installAdapter(
            imageRef = "gcr.io/project/adapter:v1.2.3",
            checksum = "sha256:abc123def456",
        )

        assertEquals(1, fakeService.installRequests.size)
        val req = fakeService.installRequests[0]
        assertEquals("gcr.io/project/adapter:v1.2.3", req.imageRef)
        assertEquals("sha256:abc123def456", req.checksum)
    }

    @Test
    fun `installAdapter returns response with job and adapter IDs`() = runTest {
        fakeService.installResponse = InstallAdapterResponse.newBuilder()
            .setJobId("job-42")
            .setAdapterId("adapter-99")
            .setState(AdapterState.ADAPTER_STATE_INSTALLING)
            .build()

        val response = client.installAdapter("img:v1", "sha256:xxx")

        assertEquals("job-42", response.jobId)
        assertEquals("adapter-99", response.adapterId)
        assertEquals(AdapterState.ADAPTER_STATE_INSTALLING, response.state)
    }

    @Test
    fun `installAdapter does not modify imageRef or checksum (Property 2)`() = runTest {
        val originalImageRef = "gcr.io/my-project/parking-adapter:v1.2.3"
        val originalChecksum = "sha256:abc123def456789"

        fakeService.installResponse = InstallAdapterResponse.newBuilder()
            .setJobId("j1")
            .setAdapterId("a1")
            .build()

        client.installAdapter(originalImageRef, originalChecksum)

        val req = fakeService.installRequests[0]
        assertEquals(
            "imageRef must be passed through unmodified",
            originalImageRef, req.imageRef
        )
        assertEquals(
            "checksum must be passed through unmodified",
            originalChecksum, req.checksum
        )
    }

    @Test
    fun `installAdapter throws ServiceException on gRPC error`() = runTest {
        fakeService.installError = Status.UNAVAILABLE
            .withDescription("Service down")
            .asException()

        try {
            client.installAdapter("img:v1", "sha256:xxx")
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertEquals("Unable to install parking adapter", e.message)
        }
    }

    // --- watchAdapterStates ---

    @Test
    fun `watchAdapterStates streams state transitions`() = runTest {
        fakeService.watchResponses = listOf(
            AdapterStateEvent.newBuilder()
                .setAdapterId("a1")
                .setOldState(AdapterState.ADAPTER_STATE_UNKNOWN)
                .setNewState(AdapterState.ADAPTER_STATE_DOWNLOADING)
                .setTimestamp(1000L)
                .build(),
            AdapterStateEvent.newBuilder()
                .setAdapterId("a1")
                .setOldState(AdapterState.ADAPTER_STATE_DOWNLOADING)
                .setNewState(AdapterState.ADAPTER_STATE_INSTALLING)
                .setTimestamp(2000L)
                .build(),
            AdapterStateEvent.newBuilder()
                .setAdapterId("a1")
                .setOldState(AdapterState.ADAPTER_STATE_INSTALLING)
                .setNewState(AdapterState.ADAPTER_STATE_RUNNING)
                .setTimestamp(3000L)
                .build(),
        )

        val updates = client.watchAdapterStates().toList()

        assertEquals(3, updates.size)

        assertEquals("a1", updates[0].adapterId)
        assertEquals(AdapterState.ADAPTER_STATE_DOWNLOADING, updates[0].newState)

        assertEquals(AdapterState.ADAPTER_STATE_INSTALLING, updates[1].newState)

        assertEquals(AdapterState.ADAPTER_STATE_RUNNING, updates[2].newState)
        assertEquals(AdapterState.ADAPTER_STATE_INSTALLING, updates[2].oldState)
        assertEquals(3000L, updates[2].timestamp)
    }

    @Test
    fun `watchAdapterStates maps error state correctly`() = runTest {
        fakeService.watchResponses = listOf(
            AdapterStateEvent.newBuilder()
                .setAdapterId("a1")
                .setOldState(AdapterState.ADAPTER_STATE_INSTALLING)
                .setNewState(AdapterState.ADAPTER_STATE_ERROR)
                .setTimestamp(5000L)
                .build(),
        )

        val update = client.watchAdapterStates().first()

        assertEquals(AdapterState.ADAPTER_STATE_ERROR, update.newState)
    }

    /**
     * Fake UpdateService for testing.
     */
    private class FakeUpdateService : UpdateServiceGrpcKt.UpdateServiceCoroutineImplBase() {

        val installRequests = mutableListOf<InstallAdapterRequest>()
        var installResponse: InstallAdapterResponse = InstallAdapterResponse.getDefaultInstance()
        var installError: Exception? = null

        var watchResponses: List<AdapterStateEvent> = emptyList()

        override suspend fun installAdapter(
            request: InstallAdapterRequest,
        ): InstallAdapterResponse {
            installRequests.add(request)
            installError?.let { throw it }
            return installResponse
        }

        override fun watchAdapterStates(
            request: WatchAdapterStatesRequest,
        ): Flow<AdapterStateEvent> {
            return flow {
                for (event in watchResponses) {
                    emit(event)
                }
            }
        }
    }
}
