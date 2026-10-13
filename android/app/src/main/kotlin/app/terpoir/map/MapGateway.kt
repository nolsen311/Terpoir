// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import java.io.IOException
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import okhttp3.Response

/**
 * The app's one way onto the network (docs/design-decisions.md,
 * "Permissions"): every map request passes here, and is refused before any
 * connection is opened unless the online map's toggle is on and the request
 * goes over HTTPS to the chosen provider's hosts.
 */
object MapGateway {
    /**
     * Sent instead of MapLibre's own User-Agent, which names the phone model
     * and Android version.
     */
    const val USER_AGENT = "Terpoir"

    /**
     * Whether a request to [scheme]://[host] may go out while the map is
     * [enabled] and [provider] is chosen.
     */
    fun allows(scheme: String, host: String, enabled: Boolean, provider: MapProvider): Boolean =
        enabled && scheme == "https" && host in provider.hosts

    /**
     * Refuses what [allows] does not, and replaces the User-Agent. Asks
     * [enabled] and [provider] on every request, so a change applies at once.
     */
    class Guard(private val enabled: () -> Boolean, private val provider: () -> MapProvider) : Interceptor {
        override fun intercept(chain: Interceptor.Chain): Response {
            val request = chain.request()
            if (!allows(request.url.scheme, request.url.host, enabled(), provider())) throw Refused()
            return chain.proceed(request.newBuilder().header("User-Agent", USER_AGENT).build())
        }
    }

    /** A request the gateway did not let out. Says nothing about the request. */
    class Refused : IOException("Refused by the map gateway")

    /**
     * The client MapLibre uses. Redirects are not followed, since they would
     * leave the checked host; nothing is cached on disk, and no cookies are
     * kept.
     */
    fun client(enabled: () -> Boolean, provider: () -> MapProvider): OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(Guard(enabled, provider))
        .followRedirects(false)
        .followSslRedirects(false)
        .build()
}
