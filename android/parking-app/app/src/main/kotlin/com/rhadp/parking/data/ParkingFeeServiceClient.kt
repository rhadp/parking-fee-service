package com.rhadp.parking.data

import android.util.Log
import com.rhadp.parking.model.AdapterMetadata
import com.rhadp.parking.model.ZoneMatch
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.IOException

/**
 * Client for the PARKING_FEE_SERVICE REST API.
 *
 * Uses OkHttp for HTTP communication and kotlinx.serialization for JSON
 * parsing. Provides zone lookup and adapter metadata retrieval.
 *
 * Requirements: 06-REQ-1.2, 06-REQ-1.4, 06-REQ-4.2
 */
class ParkingFeeServiceClient(
    private val httpClient: OkHttpClient,
    private val baseUrl: String,
) {
    private val json = Json { ignoreUnknownKeys = true }

    /**
     * Looks up parking zones near the given coordinates.
     *
     * Calls `GET /api/v1/zones?lat={lat}&lon={lon}` and parses the JSON
     * response into a list of [ZoneMatch] objects.
     *
     * @param lat the latitude coordinate.
     * @param lon the longitude coordinate.
     * @return a list of matching zones, possibly empty.
     * @throws ServiceException if the service is unreachable or returns an error.
     */
    suspend fun lookupZones(lat: Double, lon: Double): List<ZoneMatch> {
        val url = "$baseUrl/api/v1/zones?lat=$lat&lon=$lon"
        return withContext(Dispatchers.IO) {
            try {
                val request = Request.Builder()
                    .url(url)
                    .get()
                    .build()

                val response = httpClient.newCall(request).execute()
                response.use { resp ->
                    if (!resp.isSuccessful) {
                        throw ServiceException(
                            "Zone lookup failed (HTTP ${resp.code})"
                        )
                    }
                    val body = resp.body?.string()
                        ?: throw ServiceException("Empty response from zone lookup")
                    json.decodeFromString<List<ZoneMatch>>(body)
                }
            } catch (e: IOException) {
                Log.e(TAG, "Failed to reach PARKING_FEE_SERVICE: $url", e)
                throw ServiceException("Unable to reach parking service", e)
            } catch (e: ServiceException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "Unexpected error during zone lookup", e)
                throw ServiceException("Zone lookup failed", e)
            }
        }
    }

    /**
     * Retrieves adapter metadata for a parking zone.
     *
     * Calls `GET /api/v1/zones/{zoneId}/adapter` and parses the JSON
     * response into an [AdapterMetadata] object.
     *
     * @param zoneId the zone identifier.
     * @return the adapter metadata for the zone.
     * @throws ServiceException if the service is unreachable or returns an error.
     */
    suspend fun getZoneAdapter(zoneId: String): AdapterMetadata {
        val url = "$baseUrl/api/v1/zones/$zoneId/adapter"
        return withContext(Dispatchers.IO) {
            try {
                val request = Request.Builder()
                    .url(url)
                    .get()
                    .build()

                val response = httpClient.newCall(request).execute()
                response.use { resp ->
                    if (!resp.isSuccessful) {
                        throw ServiceException(
                            "Adapter metadata lookup failed (HTTP ${resp.code})"
                        )
                    }
                    val body = resp.body?.string()
                        ?: throw ServiceException("Empty response from adapter lookup")
                    json.decodeFromString<AdapterMetadata>(body)
                }
            } catch (e: IOException) {
                Log.e(TAG, "Failed to reach PARKING_FEE_SERVICE: $url", e)
                throw ServiceException("Unable to reach parking service", e)
            } catch (e: ServiceException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "Unexpected error during adapter lookup", e)
                throw ServiceException("Adapter metadata lookup failed", e)
            }
        }
    }

    companion object {
        private const val TAG = "ParkingFeeServiceClient"
    }
}
