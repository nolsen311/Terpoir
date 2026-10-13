// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import java.util.concurrent.TimeUnit
import okhttp3.Call
import okhttp3.Connection
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.Interceptor
import okhttp3.Protocol
import okhttp3.Request
import okhttp3.Response
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class MapGatewayTest {
    /** Records what the gateway lets through, without any network. */
    private class Recorder(private val request: Request) : Interceptor.Chain {
        var sent: Request? = null

        override fun request() = request

        override fun proceed(request: Request): Response {
            sent = request
            return Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(200).message("OK").build()
        }

        override fun connection(): Connection? = null

        override fun call(): Call = throw UnsupportedOperationException()

        override fun connectTimeoutMillis() = 0

        override fun withConnectTimeout(timeout: Int, unit: TimeUnit) = this

        override fun readTimeoutMillis() = 0

        override fun withReadTimeout(timeout: Int, unit: TimeUnit) = this

        override fun writeTimeoutMillis() = 0

        override fun withWriteTimeout(timeout: Int, unit: TimeUnit) = this
    }

    private fun request(url: String) = Request.Builder().url(url).header("User-Agent", "MapLibre Android/13.6.1 (Pixel 9; Android 16)").build()

    private val ofm = MapProvider.OPENFREEMAP
    private val maptiler = MapProvider.MAPTILER

    @Test
    fun onlyTheChosenProviderOverHttpsWhileOn() {
        assertTrue(MapGateway.allows("https", "tiles.openfreemap.org", enabled = true, ofm))
        assertFalse(MapGateway.allows("https", "tiles.openfreemap.org", enabled = false, ofm))
        assertFalse(MapGateway.allows("http", "tiles.openfreemap.org", enabled = true, ofm))
        assertFalse(MapGateway.allows("https", "openfreemap.org", enabled = true, ofm))
        assertFalse(MapGateway.allows("https", "tiles.openfreemap.org.example", enabled = true, ofm))
        // One provider at a time: MapTiler only while it is chosen.
        assertFalse(MapGateway.allows("https", "api.maptiler.com", enabled = true, ofm))
        assertTrue(MapGateway.allows("https", "api.maptiler.com", enabled = true, maptiler))
        assertFalse(MapGateway.allows("https", "tiles.openfreemap.org", enabled = true, maptiler))
        assertFalse(MapGateway.allows("https", "maptiler.com", enabled = true, maptiler))
        for (provider in MapProvider.entries) assertFalse(MapGateway.allows("https", "terpoir.app", enabled = true, provider))
    }

    @Test
    fun refusesEverythingWhileOff() {
        val chain = Recorder(request("https://tiles.openfreemap.org/styles/liberty"))
        assertThrows(MapGateway.Refused::class.java) { MapGateway.Guard({ false }, { ofm }).intercept(chain) }
        assertNull(chain.sent)
    }

    @Test
    fun refusesOtherHostsWhileOn() {
        val chain = Recorder(request("https://tiles.example/0/0/0.pbf"))
        assertThrows(MapGateway.Refused::class.java) { MapGateway.Guard({ true }, { ofm }).intercept(chain) }
        assertNull(chain.sent)
    }

    @Test
    fun followsAProviderChangeAtOnce() {
        var provider = maptiler
        val guard = MapGateway.Guard({ true }, { provider })
        guard.intercept(Recorder(request("https://api.maptiler.com/tiles/v3/0/0/0.pbf")))
        provider = ofm
        val chain = Recorder(request("https://api.maptiler.com/tiles/v3/0/0/0.pbf"))
        assertThrows(MapGateway.Refused::class.java) { guard.intercept(chain) }
        assertNull(chain.sent)
    }

    @Test
    fun sendsOnlyTheAppNameAsUserAgent() {
        val chain = Recorder(request("https://tiles.openfreemap.org/planet"))
        MapGateway.Guard({ true }, { ofm }).intercept(chain)
        assertEquals(listOf("Terpoir"), chain.sent?.headers("User-Agent"))
    }

    @Test
    fun theClientFollowsNoRedirects() {
        val client = MapGateway.client({ true }, { ofm })
        assertFalse(client.followRedirects)
        assertFalse(client.followSslRedirects)
        assertNull(client.cache)
    }

    @Test
    fun providersAndStyles() {
        assertEquals(MapProvider.OPENFREEMAP, MapProvider.DEFAULT)
        assertEquals(MapProvider.OPENFREEMAP, MapProvider.fromId(null))
        assertEquals(MapProvider.OPENFREEMAP, MapProvider.fromId("bing"))
        assertEquals(MapProvider.GOOGLE, MapProvider.fromId("google"))
        // Google's SDK draws the map; MapLibre gets nothing out while it is chosen.
        assertNull(MapProvider.GOOGLE.styleUrl("Key123"))
        assertFalse(MapGateway.allows("https", "tiles.openfreemap.org", enabled = true, MapProvider.GOOGLE))
        assertFalse(MapGateway.allows("https", "maps.googleapis.com", enabled = true, MapProvider.GOOGLE))
        assertEquals(MapProvider.MAPTILER, MapProvider.fromId("maptiler"))
        // Every style URL stays on its provider's hosts.
        for (provider in MapProvider.entries) {
            val url = provider.styleUrl("Key123") ?: continue
            assertTrue(url, MapGateway.allows("https", url.toHttpUrl().host, true, provider))
        }
        assertNull(MapProvider.MAPTILER.styleUrl(null))
    }
}
