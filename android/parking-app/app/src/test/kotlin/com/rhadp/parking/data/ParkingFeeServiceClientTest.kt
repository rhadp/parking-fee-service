package com.rhadp.parking.data

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * Unit tests for [ParkingFeeServiceClient].
 *
 * Uses OkHttp MockWebServer to verify correct URL construction,
 * JSON parsing, and error handling without a real PARKING_FEE_SERVICE.
 *
 * Requirements: 06-REQ-7.2, 06-REQ-7.3
 */
class ParkingFeeServiceClientTest {

    private lateinit var server: MockWebServer
    private lateinit var client: ParkingFeeServiceClient

    @Before
    fun setUp() {
        server = MockWebServer()
        server.start()

        val baseUrl = server.url("").toString().trimEnd('/')
        client = ParkingFeeServiceClient(
            httpClient = OkHttpClient(),
            baseUrl = baseUrl,
        )
    }

    @After
    fun tearDown() {
        server.shutdown()
    }

    // --- lookupZones ---

    @Test
    fun `lookupZones sends correct URL with coordinates`() = runTest {
        server.enqueue(MockResponse().setBody("[]").setResponseCode(200))

        client.lookupZones(48.137154, 11.576124)

        val request = server.takeRequest()
        assertEquals("GET", request.method)
        assertTrue(
            "URL should contain lat parameter",
            request.path!!.contains("lat=48.137154")
        )
        assertTrue(
            "URL should contain lon parameter",
            request.path!!.contains("lon=11.576124")
        )
        assertTrue(
            "URL should start with /api/v1/zones",
            request.path!!.startsWith("/api/v1/zones")
        )
    }

    @Test
    fun `lookupZones parses zone list correctly`() = runTest {
        val json = """
            [
              {
                "zone_id": "zone-marienplatz",
                "name": "Marienplatz Central",
                "operator_name": "Munich Parking AG",
                "rate_type": "per_minute",
                "rate_amount": 0.05,
                "currency": "EUR",
                "distance_meters": 120.5
              },
              {
                "zone_id": "zone-viktualien",
                "name": "Viktualienmarkt",
                "operator_name": "City Parking",
                "rate_type": "per_hour",
                "rate_amount": 3.00,
                "currency": "EUR",
                "distance_meters": 350.0
              }
            ]
        """.trimIndent()

        server.enqueue(MockResponse().setBody(json).setResponseCode(200))

        val zones = client.lookupZones(48.137154, 11.576124)

        assertEquals(2, zones.size)

        val first = zones[0]
        assertEquals("zone-marienplatz", first.zoneId)
        assertEquals("Marienplatz Central", first.name)
        assertEquals("Munich Parking AG", first.operatorName)
        assertEquals("per_minute", first.rateType)
        assertEquals(0.05, first.rateAmount, 0.001)
        assertEquals("EUR", first.currency)
        assertEquals(120.5, first.distanceMeters, 0.1)

        val second = zones[1]
        assertEquals("zone-viktualien", second.zoneId)
        assertEquals("Viktualienmarkt", second.name)
    }

    @Test
    fun `lookupZones returns empty list when no zones found`() = runTest {
        server.enqueue(MockResponse().setBody("[]").setResponseCode(200))

        val zones = client.lookupZones(0.0, 0.0)

        assertTrue(zones.isEmpty())
    }

    @Test
    fun `lookupZones throws ServiceException on HTTP error`() = runTest {
        server.enqueue(MockResponse().setResponseCode(500).setBody("Internal Error"))

        try {
            client.lookupZones(48.137, 11.576)
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertTrue(e.message!!.contains("Zone lookup failed"))
        }
    }

    @Test
    fun `lookupZones throws ServiceException when server unreachable`() = runTest {
        // Use a client pointing to a server that's already shut down
        server.shutdown()

        val deadClient = ParkingFeeServiceClient(
            httpClient = OkHttpClient(),
            baseUrl = "http://localhost:1",
        )

        try {
            deadClient.lookupZones(48.137, 11.576)
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertEquals("Unable to reach parking service", e.message)
        }
    }

    @Test
    fun `lookupZones ignores unknown JSON fields`() = runTest {
        val json = """
            [
              {
                "zone_id": "z1",
                "name": "Test Zone",
                "operator_name": "Test Op",
                "rate_type": "flat",
                "rate_amount": 1.0,
                "currency": "USD",
                "distance_meters": 0.0,
                "unknown_field": "ignored"
              }
            ]
        """.trimIndent()

        server.enqueue(MockResponse().setBody(json).setResponseCode(200))

        val zones = client.lookupZones(0.0, 0.0)

        assertEquals(1, zones.size)
        assertEquals("z1", zones[0].zoneId)
    }

    // --- getZoneAdapter ---

    @Test
    fun `getZoneAdapter sends correct URL with zone ID`() = runTest {
        val json = """
            {
              "zone_id": "zone-123",
              "image_ref": "gcr.io/project/adapter:v1",
              "checksum": "sha256:abc123"
            }
        """.trimIndent()

        server.enqueue(MockResponse().setBody(json).setResponseCode(200))

        client.getZoneAdapter("zone-123")

        val request = server.takeRequest()
        assertEquals("GET", request.method)
        assertEquals("/api/v1/zones/zone-123/adapter", request.path)
    }

    @Test
    fun `getZoneAdapter parses adapter metadata correctly`() = runTest {
        val json = """
            {
              "zone_id": "zone-marienplatz",
              "image_ref": "gcr.io/my-project/parking-adapter:v1.2.3",
              "checksum": "sha256:abc123def456"
            }
        """.trimIndent()

        server.enqueue(MockResponse().setBody(json).setResponseCode(200))

        val metadata = client.getZoneAdapter("zone-marienplatz")

        assertEquals("zone-marienplatz", metadata.zoneId)
        assertEquals("gcr.io/my-project/parking-adapter:v1.2.3", metadata.imageRef)
        assertEquals("sha256:abc123def456", metadata.checksum)
    }

    @Test
    fun `getZoneAdapter throws ServiceException on 404`() = runTest {
        server.enqueue(
            MockResponse()
                .setResponseCode(404)
                .setBody("""{"error": "zone not found"}""")
        )

        try {
            client.getZoneAdapter("nonexistent")
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertTrue(e.message!!.contains("Adapter metadata lookup failed"))
        }
    }

    @Test
    fun `getZoneAdapter throws ServiceException when server unreachable`() = runTest {
        server.shutdown()

        val deadClient = ParkingFeeServiceClient(
            httpClient = OkHttpClient(),
            baseUrl = "http://localhost:1",
        )

        try {
            deadClient.getZoneAdapter("zone-123")
            assertTrue("Expected ServiceException", false)
        } catch (e: ServiceException) {
            assertEquals("Unable to reach parking service", e.message)
        }
    }
}
