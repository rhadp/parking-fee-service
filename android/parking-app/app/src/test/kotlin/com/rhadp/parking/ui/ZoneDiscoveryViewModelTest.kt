package com.rhadp.parking.ui

import com.rhadp.parking.data.DataBrokerClient
import com.rhadp.parking.data.ParkingFeeServiceClient
import com.rhadp.parking.data.ServiceException
import com.rhadp.parking.data.UpdateServiceClient
import com.rhadp.parking.model.AdapterMetadata
import com.rhadp.parking.model.Location
import com.rhadp.parking.model.ZoneMatch
import com.rhadp.parking.ui.zone.ZoneDiscoveryViewModel
import com.rhadp.parking.ui.zone.ZoneDiscoveryViewModel.UiState
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import parking.common.Common.AdapterState
import parking.services.update.UpdateServiceOuterClass.InstallAdapterResponse

/**
 * Unit tests for [ZoneDiscoveryViewModel].
 *
 * Verifies state transitions and service call sequences for zone discovery:
 * - Loading → ZonesFound (normal case)
 * - Loading → NoZones (empty result)
 * - Loading → Error (DATA_BROKER unreachable)
 * - Loading → Error (PFS unreachable)
 * - ZonesFound → Installing (zone selected, adapter installed)
 *
 * Tests Property 1 (Location-to-Zone Pipeline): coordinates from DATA_BROKER
 * are passed to PFS without modification.
 *
 * Tests Property 2 (Adapter Install Trigger): imageRef and checksum from PFS
 * metadata are passed to UPDATE_SERVICE without modification.
 *
 * Tests Property 4 (Error Visibility): all service errors produce Error state.
 *
 * Requirements: 06-REQ-1.1–1.4, 06-REQ-1.E1–1.E3, 06-REQ-7.1, 06-REQ-7.3
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ZoneDiscoveryViewModelTest {

    private val testDispatcher = StandardTestDispatcher()

    private lateinit var dataBrokerClient: DataBrokerClient
    private lateinit var pfsClient: ParkingFeeServiceClient
    private lateinit var updateServiceClient: UpdateServiceClient

    private lateinit var viewModel: ZoneDiscoveryViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        dataBrokerClient = mockk()
        pfsClient = mockk()
        updateServiceClient = mockk()
        viewModel = ZoneDiscoveryViewModel(dataBrokerClient, pfsClient, updateServiceClient)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // --- loadZones: success cases ---

    @Test
    fun `loadZones transitions to ZonesFound when zones available`() = runTest {
        val location = Location(48.137154, 11.576124)
        val zones = listOf(
            ZoneMatch("z1", "Zone 1", "Op 1", "per_minute", 0.05, "EUR", 100.0),
            ZoneMatch("z2", "Zone 2", "Op 2", "per_hour", 3.0, "EUR", 200.0),
        )
        coEvery { dataBrokerClient.getLocation() } returns location
        coEvery { pfsClient.lookupZones(48.137154, 11.576124) } returns zones

        viewModel.loadZones()
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected ZonesFound, got $state", state is UiState.ZonesFound)
        assertEquals(2, (state as UiState.ZonesFound).zones.size)
        assertEquals("z1", state.zones[0].zoneId)
        assertEquals("z2", state.zones[1].zoneId)
    }

    @Test
    fun `loadZones passes exact coordinates from DataBroker to PFS (Property 1)`() = runTest {
        val lat = 48.137154
        val lon = 11.576124
        coEvery { dataBrokerClient.getLocation() } returns Location(lat, lon)
        coEvery { pfsClient.lookupZones(lat, lon) } returns emptyList()

        viewModel.loadZones()
        advanceUntilIdle()

        coVerify(exactly = 1) { pfsClient.lookupZones(lat, lon) }
    }

    @Test
    fun `loadZones transitions to NoZones when empty list returned`() = runTest {
        coEvery { dataBrokerClient.getLocation() } returns Location(0.0, 0.0)
        coEvery { pfsClient.lookupZones(0.0, 0.0) } returns emptyList()

        viewModel.loadZones()
        advanceUntilIdle()

        assertEquals(UiState.NoZones, viewModel.uiState.value)
    }

    // --- loadZones: error cases (Property 4: Error Visibility) ---

    @Test
    fun `loadZones transitions to Error when DataBroker unreachable`() = runTest {
        coEvery { dataBrokerClient.getLocation() } throws
            ServiceException("Unable to read vehicle location")

        viewModel.loadZones()
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertEquals(
            "Unable to read vehicle location",
            (state as UiState.Error).message,
        )
    }

    @Test
    fun `loadZones transitions to Error when PFS unreachable`() = runTest {
        coEvery { dataBrokerClient.getLocation() } returns Location(48.0, 11.0)
        coEvery { pfsClient.lookupZones(48.0, 11.0) } throws
            ServiceException("Unable to reach parking service")

        viewModel.loadZones()
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertEquals(
            "Unable to reach parking service",
            (state as UiState.Error).message,
        )
    }

    @Test
    fun `loadZones transitions to Error when location is null`() = runTest {
        coEvery { dataBrokerClient.getLocation() } returns null

        viewModel.loadZones()
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertEquals(
            "Vehicle location not available",
            (state as UiState.Error).message,
        )
    }

    @Test
    fun `loadZones starts in Loading state`() = runTest {
        coEvery { dataBrokerClient.getLocation() } returns Location(0.0, 0.0)
        coEvery { pfsClient.lookupZones(any(), any()) } returns emptyList()

        // Verify initial state is Loading
        assertEquals(UiState.Loading, viewModel.uiState.value)
    }

    // --- selectZone: success cases ---

    @Test
    fun `selectZone transitions to Installing on success`() = runTest {
        val zone = ZoneMatch("z1", "Zone 1", "Op 1", "per_minute", 0.05, "EUR", 100.0)
        val metadata = AdapterMetadata("z1", "gcr.io/proj/adapter:v1", "sha256:abc")
        val installResponse = InstallAdapterResponse.newBuilder()
            .setJobId("job-1")
            .setAdapterId("adapter-1")
            .setState(AdapterState.ADAPTER_STATE_INSTALLING)
            .build()

        coEvery { pfsClient.getZoneAdapter("z1") } returns metadata
        coEvery {
            updateServiceClient.installAdapter("gcr.io/proj/adapter:v1", "sha256:abc")
        } returns installResponse

        viewModel.selectZone(zone)
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Installing, got $state", state is UiState.Installing)
        assertEquals("z1", (state as UiState.Installing).zoneId)
        assertEquals("adapter-1", state.adapterId)
    }

    @Test
    fun `selectZone passes exact metadata to UpdateService (Property 2)`() = runTest {
        val imageRef = "gcr.io/my-project/parking-adapter:v1.2.3"
        val checksum = "sha256:abc123def456789"
        val zone = ZoneMatch("z1", "Zone 1", "Op 1", "per_minute", 0.05, "EUR", 100.0)
        val metadata = AdapterMetadata("z1", imageRef, checksum)

        coEvery { pfsClient.getZoneAdapter("z1") } returns metadata
        coEvery {
            updateServiceClient.installAdapter(imageRef, checksum)
        } returns InstallAdapterResponse.getDefaultInstance()

        viewModel.selectZone(zone)
        advanceUntilIdle()

        coVerify(exactly = 1) {
            updateServiceClient.installAdapter(imageRef, checksum)
        }
    }

    // --- selectZone: error cases ---

    @Test
    fun `selectZone transitions to Error when PFS getZoneAdapter fails`() = runTest {
        val zone = ZoneMatch("z1", "Zone 1", "Op 1", "per_minute", 0.05, "EUR", 100.0)
        coEvery { pfsClient.getZoneAdapter("z1") } throws
            ServiceException("Adapter metadata lookup failed (HTTP 404)")

        viewModel.selectZone(zone)
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertTrue((state as UiState.Error).message.contains("Adapter metadata lookup failed"))
    }

    @Test
    fun `selectZone transitions to Error when UpdateService install fails`() = runTest {
        val zone = ZoneMatch("z1", "Zone 1", "Op 1", "per_minute", 0.05, "EUR", 100.0)
        val metadata = AdapterMetadata("z1", "img:v1", "sha256:x")

        coEvery { pfsClient.getZoneAdapter("z1") } returns metadata
        coEvery {
            updateServiceClient.installAdapter("img:v1", "sha256:x")
        } throws ServiceException("Unable to install parking adapter")

        viewModel.selectZone(zone)
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertTrue("Expected Error, got $state", state is UiState.Error)
        assertEquals(
            "Unable to install parking adapter",
            (state as UiState.Error).message,
        )
    }
}
