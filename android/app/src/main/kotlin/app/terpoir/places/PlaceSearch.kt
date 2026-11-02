// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.places

import android.content.Context
import android.location.Address
import android.location.Geocoder
import android.os.Build
import app.terpoir.core.PlaceData
import app.terpoir.core.parsePlaceSearch
import app.terpoir.map.MapGateway
import java.io.IOException
import kotlin.coroutines.resume
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import okhttp3.HttpUrl
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response

/**
 * Address search's only way onto the network (docs/design-decisions.md,
 * "Address search"): every request is refused before any connection is
 * opened unless the toggle is on and the request goes over HTTPS to Photon.
 */
object SearchGateway {
    /** Photon, komoot's OpenStreetMap search. */
    const val HOST = "photon.komoot.io"

    /** Whether a request to [scheme]://[host] may go out while the toggle is [enabled]. */
    fun allows(scheme: String, host: String, enabled: Boolean): Boolean = enabled && scheme == "https" && host == HOST

    /** Refuses what [allows] does not, and sends the app's plain User-Agent. */
    class Guard(private val enabled: () -> Boolean) : Interceptor {
        override fun intercept(chain: Interceptor.Chain): Response {
            val request = chain.request()
            if (!allows(request.url.scheme, request.url.host, enabled())) throw Refused()
            return chain.proceed(request.newBuilder().header("User-Agent", MapGateway.USER_AGENT).build())
        }
    }

    /** A request the gateway did not let out. Says nothing about the request. */
    class Refused : IOException("Refused by the search gateway")

    /** No redirects (they would leave the checked host), no disk cache, no cookies. */
    fun client(enabled: () -> Boolean): OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(Guard(enabled))
        .followRedirects(false)
        .followSslRedirects(false)
        .build()
}

/**
 * Searches for a place by address or name, only when the user asks: on
 * OpenStreetMap through Photon, and with the phone's own geocoder where it
 * has one. Never sends the phone's location. [enabled] is the toggle.
 */
class PlaceSearch(private val context: Context, private val enabled: () -> Boolean) {
    private val client by lazy { SearchGateway.client(enabled) }

    /** Photon's places for [query]; throws IOException when the search fails. */
    suspend fun openStreetMap(query: String): List<PlaceData> = withContext(Dispatchers.IO) {
        val url = HttpUrl.Builder()
            .scheme("https")
            .host(SearchGateway.HOST)
            .addPathSegment("api")
            .addQueryParameter("q", query)
            .addQueryParameter("limit", "10")
            .build()
        client.newCall(Request.Builder().url(url).build()).execute().use { response ->
            if (!response.isSuccessful) throw IOException("Search failed: ${response.code}")
            parsePlaceSearch(response.body.string())
        }
    }

    /** The phone's geocoder's places for [query]; none when it has no geocoder or it fails. */
    suspend fun phone(query: String): List<PlaceData> {
        if (!enabled() || !Geocoder.isPresent()) return emptyList()
        val geocoder = Geocoder(context)
        val addresses: List<Address> = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            suspendCancellableCoroutine { done ->
                geocoder.getFromLocationName(
                    query,
                    MAX_RESULTS,
                    object : Geocoder.GeocodeListener {
                        override fun onGeocode(addresses: MutableList<Address>) = done.resume(addresses)

                        override fun onError(errorMessage: String?) = done.resume(emptyList())
                    },
                )
            }
        } else {
            withContext(Dispatchers.IO) {
                @Suppress("DEPRECATION")
                runCatching { geocoder.getFromLocationName(query, MAX_RESULTS) }.getOrNull().orEmpty()
            }
        }
        return addresses.mapNotNull(::place)
    }

    private fun place(address: Address): PlaceData? {
        if (!address.hasLatitude() || !address.hasLongitude()) return null
        val line = address.getAddressLine(0)?.trim().orEmpty()
        val street = listOfNotNull(address.subThoroughfare, address.thoroughfare).joinToString(" ").trim()
        // The feature name is the house number for a plain address.
        val feature = address.featureName?.trim()?.takeIf { it.isNotEmpty() && it != address.subThoroughfare }
        val name = feature ?: street.ifEmpty { line }.ifEmpty { return null }
        val detail = line.removePrefix(name).trimStart(',', ' ')
        val accuracy = when {
            address.subThoroughfare != null -> 25
            address.thoroughfare != null -> 200
            address.locality != null -> 2_000
            else -> 20_000
        }
        return PlaceData(name, detail, (address.latitude * 1e7).toInt(), (address.longitude * 1e7).toInt(), accuracy.toUInt())
    }

    private companion object {
        const val MAX_RESULTS = 5
    }
}
