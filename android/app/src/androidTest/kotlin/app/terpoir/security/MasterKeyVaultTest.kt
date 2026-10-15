// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.security

import android.hardware.biometrics.BiometricManager
import android.os.Build
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.security.KeyStore
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Assume.assumeFalse
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The wrapping key is hardware-backed and crypto-bound to user
 * authentication. Needs a secure lock screen on the test device (CI sets a
 * PIN before running).
 */
@RunWith(AndroidJUnit4::class)
class MasterKeyVaultTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val wrappedFile = context.noBackupFilesDir.resolve("vault-test/master")
    private val vault = MasterKeyVault(wrappedFile)

    @After
    fun cleanUp() {
        vault.destroy()
    }

    private fun keyInfo(): KeyInfo {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val key = keyStore.getKey(MasterKeyVault.ALIAS, null) as SecretKey
        return SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore")
            .getKeySpec(key, KeyInfo::class.java) as KeyInfo
    }

    @Test
    fun defaultKeyRequiresBiometricOrCredentialOnEveryUse() {
        vault.createWrappingKey(strict = false)
        val info = keyInfo()
        assertTrue(info.isUserAuthenticationRequired)
        assertEquals(0, info.userAuthenticationValidityDurationSeconds)
        assertEquals(
            KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
            info.userAuthenticationType,
        )
        assertFalse(info.isInvalidatedByBiometricEnrollment)
    }

    @Test
    fun keyIsInSecureHardwareOnRealDevices() {
        // Emulators have no TEE or StrongBox.
        assumeFalse(isEmulator())
        vault.createWrappingKey(strict = false)
        assertTrue(keyInfo().isInsideSecureHardware)
    }

    @Test
    fun strictKeyIsBiometricOnlyAndInvalidatedByEnrollment() {
        // Needs an enrolled strong biometric.
        val biometrics = context.getSystemService(BiometricManager::class.java)
        assumeTrue(
            biometrics.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) ==
                BiometricManager.BIOMETRIC_SUCCESS,
        )
        vault.createWrappingKey(strict = true)
        val info = keyInfo()
        assertEquals(KeyProperties.AUTH_BIOMETRIC_STRONG, info.userAuthenticationType)
        assertTrue(info.isInvalidatedByBiometricEnrollment)
    }

    @Test
    fun keyIsUnusableWithoutAuthentication() {
        vault.createWrappingKey(strict = false)
        val cipher = vault.encryptCipher()
        try {
            cipher.doFinal(ByteArray(32))
            fail("the wrapping key worked without user authentication")
        } catch (_: Exception) {
            // Expected: the Keystore refuses until the user authenticates.
        }
    }

    @Test
    fun unlockingAgainReusesTheOpenCipher() {
        // Each cancelled unlock prompt used to leave a Keystore operation
        // open until garbage collection; enough of them and the Keystore
        // refused new ones until the phone restarted.
        vault.createWrappingKey(strict = false)
        wrappedFile.parentFile?.mkdirs()
        wrappedFile.writeBytes(WrappedMasterKey(ByteArray(12), ByteArray(48)).toBytes())
        val first = vault.decryptCipher()
        repeat(200) { assertSame(first, vault.decryptCipher()) }
        // A new wrapping key starts a new cipher.
        vault.createWrappingKey(strict = false)
        assertNotSame(first, vault.decryptCipher())
    }

    private fun isEmulator(): Boolean =
        Build.HARDWARE.contains("ranchu") || Build.HARDWARE.contains("goldfish")
}
