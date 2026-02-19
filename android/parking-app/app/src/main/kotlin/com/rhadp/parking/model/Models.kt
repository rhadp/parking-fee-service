package com.rhadp.parking.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Vehicle location from DATA_BROKER.
 */
data class Location(
    val latitude: Double,
    val longitude: Double,
)

/**
 * A parking zone matched by proximity search.
 *
 * Serializable for JSON parsing from PARKING_FEE_SERVICE REST responses.
 */
@Serializable
data class ZoneMatch(
    @SerialName("zone_id") val zoneId: String,
    val name: String,
    @SerialName("operator_name") val operatorName: String,
    @SerialName("rate_type") val rateType: String,
    @SerialName("rate_amount") val rateAmount: Double,
    val currency: String,
    @SerialName("distance_meters") val distanceMeters: Double,
)

/**
 * Adapter container metadata for a parking zone.
 *
 * Serializable for JSON parsing from PARKING_FEE_SERVICE REST responses.
 */
@Serializable
data class AdapterMetadata(
    @SerialName("zone_id") val zoneId: String,
    @SerialName("image_ref") val imageRef: String,
    val checksum: String,
)

/**
 * Parking session information from PARKING_OPERATOR_ADAPTOR.
 */
data class SessionInfo(
    val sessionId: String,
    val active: Boolean,
    val startTime: Long,
    val currentFee: Double,
)
