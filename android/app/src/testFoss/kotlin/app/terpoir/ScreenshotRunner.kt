// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import java.io.InputStream
import java.io.OutputStream
import java.security.Key
import java.security.KeyStore
import java.security.KeyStoreSpi
import java.security.Provider
import java.security.Security
import java.security.cert.Certificate
import java.util.Date
import java.util.Enumeration
import org.robolectric.RobolectricTestRunner

/**
 * Robolectric, with an in-memory stand-in for the Android Keystore, which
 * the JVM doesn't have. The app opens it when it starts (MasterKeyVault),
 * before a test can step in; StoreScreenshots opens its journal with keys
 * of its own and never unlocks through it.
 */
class ScreenshotRunner(testClass: Class<*>) : RobolectricTestRunner(testClass) {
    init {
        if (Security.getProvider(PROVIDER) == null) {
            Security.addProvider(
                object : Provider(PROVIDER, 1.0, "In-memory Android Keystore for StoreScreenshots") {
                    init {
                        put("KeyStore.$PROVIDER", InMemoryKeyStore::class.java.name)
                    }
                },
            )
        }
    }

    private companion object {
        const val PROVIDER = "AndroidKeyStore"
    }
}

/** A PKCS#12 key store kept in memory, under the Android Keystore's name. */
class InMemoryKeyStore : KeyStoreSpi() {
    private val store: KeyStore = KeyStore.getInstance("PKCS12").apply { load(null) }

    override fun engineGetKey(alias: String, password: CharArray?): Key? = store.getKey(alias, password)

    override fun engineGetCertificateChain(alias: String): Array<Certificate>? = store.getCertificateChain(alias)

    override fun engineGetCertificate(alias: String): Certificate? = store.getCertificate(alias)

    override fun engineGetCreationDate(alias: String): Date? = store.getCreationDate(alias)

    override fun engineSetKeyEntry(alias: String, key: Key, password: CharArray?, chain: Array<Certificate>?) =
        store.setKeyEntry(alias, key, password, chain)

    override fun engineSetKeyEntry(alias: String, key: ByteArray, chain: Array<Certificate>?) =
        store.setKeyEntry(alias, key, chain)

    override fun engineSetCertificateEntry(alias: String, cert: Certificate) = store.setCertificateEntry(alias, cert)

    override fun engineDeleteEntry(alias: String) = store.deleteEntry(alias)

    override fun engineAliases(): Enumeration<String> = store.aliases()

    override fun engineContainsAlias(alias: String): Boolean = store.containsAlias(alias)

    override fun engineSize(): Int = store.size()

    override fun engineIsKeyEntry(alias: String): Boolean = store.isKeyEntry(alias)

    override fun engineIsCertificateEntry(alias: String): Boolean = store.isCertificateEntry(alias)

    override fun engineGetCertificateAlias(cert: Certificate): String? = store.getCertificateAlias(cert)

    override fun engineStore(stream: OutputStream?, password: CharArray?) = Unit

    override fun engineLoad(stream: InputStream?, password: CharArray?) = Unit
}
