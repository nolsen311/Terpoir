// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import java.util.concurrent.TimeUnit
import okhttp3.Call
import okhttp3.Connection
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

    @Test
    fun onlyTheTileProviderOverHttpsWhileOn() {
        assertTrue(MapGateway.allows("https", "api.maptiler.com", enabled = true))
        assertFalse(MapGateway.allows("https", "api.maptiler.com", enabled = false))
        assertFalse(MapGateway.allows("http", "api.maptiler.com", enabled = true))
        assertFalse(MapGateway.allows("https", "maptiler.com", enabled = true))
        assertFalse(MapGateway.allows("https", "api.maptiler.com.example", enabled = true))
        assertFalse(MapGateway.allows("https", "terpoir.app", enabled = true))
    }

    @Test
    fun refusesEverythingWhileOff() {
        val chain = Recorder(request("https://api.maptiler.com/maps/streets-v2/style.json"))
        assertThrows(MapGateway.Refused::class.java) { MapGateway.Guard { false }.intercept(chain) }
        assertNull(chain.sent)
    }

    @Test
    fun refusesOtherHostsWhileOn() {
        val chain = Recorder(request("https://tiles.example/0/0/0.pbf"))
        assertThrows(MapGateway.Refused::class.java) { MapGateway.Guard { true }.intercept(chain) }
        assertNull(chain.sent)
    }

    @Test
    fun sendsOnlyTheAppNameAsUserAgent() {
        val chain = Recorder(request("https://api.maptiler.com/tiles/v3/0/0/0.pbf"))
        MapGateway.Guard { true }.intercept(chain)
        assertEquals(listOf("Terpoir"), chain.sent?.headers("User-Agent"))
    }

    @Test
    fun theClientFollowsNoRedirects() {
        val client = MapGateway.client { true }
        assertFalse(client.followRedirects)
        assertFalse(client.followSslRedirects)
        assertNull(client.cache)
    }
}
