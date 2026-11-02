// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.places

import java.lang.reflect.Proxy
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

class SearchGatewayTest {
    /** Records what the gateway lets through, without any network (as MapGatewayTest does). */
    private class Recorder(private val request: Request) {
        var sent: Request? = null

        val chain: Interceptor.Chain = Proxy.newProxyInstance(
            Interceptor.Chain::class.java.classLoader,
            arrayOf(Interceptor.Chain::class.java),
        ) { _, method, args ->
            when (method.name) {
                "request" -> request
                "proceed" -> {
                    val sending = args!![0] as Request
                    sent = sending
                    Response.Builder().request(sending).protocol(Protocol.HTTP_1_1).code(200).message("OK").build()
                }
                else -> error("unexpected Chain.${method.name}")
            }
        } as Interceptor.Chain
    }

    private fun request(url: String) = Request.Builder().url(url.toHttpUrl()).header("User-Agent", "okhttp/5").build()

    @Test
    fun onlyPhotonOverHttpsWhileOn() {
        assertTrue(SearchGateway.allows("https", "photon.komoot.io", enabled = true))
        assertFalse(SearchGateway.allows("https", "photon.komoot.io", enabled = false))
        assertFalse(SearchGateway.allows("http", "photon.komoot.io", enabled = true))
        for (host in listOf("nominatim.openstreetmap.org", "komoot.io", "photon.komoot.io.example.com", "tiles.openfreemap.org")) {
            assertFalse(host, SearchGateway.allows("https", host, enabled = true))
        }
    }

    @Test
    fun aSearchGoesOutWithTheAppsPlainUserAgent() {
        val recorder = Recorder(request("https://photon.komoot.io/api?q=Ala+Moana&limit=10"))
        SearchGateway.Guard { true }.intercept(recorder.chain)
        assertEquals("Terpoir", recorder.sent?.header("User-Agent"))
    }

    @Test
    fun nothingLeavesWhileOffOrToAnotherHost() {
        for ((url, on) in listOf("https://photon.komoot.io/api?q=x" to false, "https://example.com/api?q=x" to true)) {
            val recorder = Recorder(request(url))
            assertThrows(SearchGateway.Refused::class.java) { SearchGateway.Guard { on }.intercept(recorder.chain) }
            assertNull(recorder.sent)
        }
    }
}
