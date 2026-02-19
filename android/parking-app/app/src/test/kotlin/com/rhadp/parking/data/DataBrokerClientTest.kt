package com.rhadp.parking.data

import com.google.protobuf.Timestamp
import io.grpc.ManagedChannel
import io.grpc.Status
import io.grpc.StatusException
import io.grpc.inprocess.InProcessChannelBuilder
import io.grpc.inprocess.InProcessServerBuilder
import io.grpc.testing.GrpcCleanupRule
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import kuksa.`val`.v2.Types.Datapoint
import kuksa.`val`.v2.Types.Value
import kuksa.`val`.v2.Val.GetValueRequest
import kuksa.`val`.v2.Val.GetValueResponse
import kuksa.`val`.v2.Val.SubscribeRequest
import kuksa.`val`.v2.Val.SubscribeResponse
import kuksa.`val`.v2.VALGrpcKt
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * Unit tests for [DataBrokerClient].
 *
 * Uses grpc-testing InProcessServer to verify correct gRPC call
 * construction and response parsing without a real DATA_BROKER.
 *
 * Requirements: 06-REQ-7.2, 06-REQ-7.3
 */
class DataBrokerClientTest {

    @get:Rule
    val grpcCleanup = GrpcCleanupRule()

    private lateinit var channel: ManagedChannel
    private lateinit var client: DataBrokerClient

    private lateinit var fakeService: FakeVALService

    @Before
    fun setUp() {
        fakeService = FakeVALService()

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
        client = DataBrokerClient(channel)
    }

    @Test
    fun `getLocation returns location when signals available`() = runTest {
        fakeService.valueResponses["Vehicle.CurrentLocation.Latitude"] =
            makeDoubleResponse(48.137154)
        fakeService.valueResponses["Vehicle.CurrentLocation.Longitude"] =
            makeDoubleResponse(11.576124)

        val location = client.getLocation()

        assertNotNull(location)
        assertEquals(48.137154, location!!.latitude, 0.000001)
        assertEquals(11.576124, location.longitude, 0.000001)
    }

    @Test
    fun `getLocation requests correct signal paths`() = runTest {
        fakeService.valueResponses["Vehicle.CurrentLocation.Latitude"] =
            makeDoubleResponse(48.0)
        fakeService.valueResponses["Vehicle.CurrentLocation.Longitude"] =
            makeDoubleResponse(11.0)

        client.getLocation()

        assertEquals(2, fakeService.getValueRequests.size)
        assertEquals(
            "Vehicle.CurrentLocation.Latitude",
            fakeService.getValueRequests[0].signalId.path
        )
        assertEquals(
            "Vehicle.CurrentLocation.Longitude",
            fakeService.getValueRequests[1].signalId.path
        )
    }

    @Test
    fun `getLocation returns null when latitude has no value`() = runTest {
        fakeService.valueResponses["Vehicle.CurrentLocation.Latitude"] =
            GetValueResponse.getDefaultInstance()
        fakeService.valueResponses["Vehicle.CurrentLocation.Longitude"] =
            makeDoubleResponse(11.0)

        val location = client.getLocation()

        assertNull(location)
    }

    @Test
    fun `getLocation throws ServiceException when broker unreachable`() = runTest {
        fakeService.getValueError = Status.UNAVAILABLE.asException()

        try {
            client.getLocation()
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertEquals("Unable to read vehicle location", e.message)
        }
    }

    @Test
    fun `subscribeSessionActive emits true when signal is true`() = runTest {
        fakeService.subscribeResponses = listOf(
            makeSubscribeResponse("Vehicle.Parking.SessionActive", true)
        )

        val result = client.subscribeSessionActive().first()

        assertTrue(result)
    }

    @Test
    fun `subscribeSessionActive emits false when signal is false`() = runTest {
        fakeService.subscribeResponses = listOf(
            makeSubscribeResponse("Vehicle.Parking.SessionActive", false)
        )

        val result = client.subscribeSessionActive().first()

        assertEquals(false, result)
    }

    @Test
    fun `subscribeSessionActive streams multiple state changes`() = runTest {
        fakeService.subscribeResponses = listOf(
            makeSubscribeResponse("Vehicle.Parking.SessionActive", true),
            makeSubscribeResponse("Vehicle.Parking.SessionActive", false),
            makeSubscribeResponse("Vehicle.Parking.SessionActive", true),
        )

        val results = client.subscribeSessionActive().toList()

        assertEquals(listOf(true, false, true), results)
    }

    @Test
    fun `subscribeSessionActive requests correct signal path`() = runTest {
        fakeService.subscribeResponses = listOf(
            makeSubscribeResponse("Vehicle.Parking.SessionActive", true)
        )

        client.subscribeSessionActive().first()

        assertEquals(1, fakeService.subscribeRequests.size)
        assertTrue(
            fakeService.subscribeRequests[0].signalPathsList
                .contains("Vehicle.Parking.SessionActive")
        )
    }

    // --- Helpers ---

    private fun makeDoubleResponse(value: Double): GetValueResponse {
        return GetValueResponse.newBuilder()
            .setDataPoint(
                Datapoint.newBuilder()
                    .setTimestamp(Timestamp.getDefaultInstance())
                    .setValue(
                        Value.newBuilder()
                            .setDouble(value)
                            .build()
                    )
                    .build()
            )
            .build()
    }

    private fun makeSubscribeResponse(
        path: String,
        boolValue: Boolean,
    ): SubscribeResponse {
        return SubscribeResponse.newBuilder()
            .putEntries(
                path,
                Datapoint.newBuilder()
                    .setTimestamp(Timestamp.getDefaultInstance())
                    .setValue(
                        Value.newBuilder()
                            .setBool(boolValue)
                            .build()
                    )
                    .build()
            )
            .build()
    }

    /**
     * Fake VAL service for testing.
     *
     * Stores requests for verification and returns configurable responses.
     */
    private class FakeVALService : VALGrpcKt.VALCoroutineImplBase() {

        val valueResponses = mutableMapOf<String, GetValueResponse>()
        val getValueRequests = mutableListOf<GetValueRequest>()
        val subscribeRequests = mutableListOf<SubscribeRequest>()
        var subscribeResponses: List<SubscribeResponse> = emptyList()
        var getValueError: Exception? = null

        override suspend fun getValue(request: GetValueRequest): GetValueResponse {
            getValueRequests.add(request)
            getValueError?.let { throw it }
            val path = request.signalId.path
            return valueResponses[path]
                ?: throw Status.NOT_FOUND
                    .withDescription("Signal not found: $path")
                    .asException()
        }

        override fun subscribe(request: SubscribeRequest): Flow<SubscribeResponse> {
            subscribeRequests.add(request)
            return flow {
                for (response in subscribeResponses) {
                    emit(response)
                }
            }
        }
    }
}
