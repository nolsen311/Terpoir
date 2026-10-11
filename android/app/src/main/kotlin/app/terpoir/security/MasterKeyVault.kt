// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.security

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Keeps the journal's master key wrapped by a hardware-backed Android
 * Keystore key (StrongBox where the device has it).
 *
 * The wrapping key is **crypto-bound to user authentication**: every use
 * needs a fresh BiometricPrompt confirmation (strong biometric or the
 * device credential; biometric only in strict mode), so a patched app
 * cannot unwrap the master key without the user. It also works only while
 * the device is unlocked.
 *
 * This class only wraps and unwraps bytes; the master key itself is created
 * and used by the Rust core.
 */
class MasterKeyVault(private val wrappedFile: File) {
    private val keyStore: KeyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }

    /** Whether a wrapped master key and its wrapping key both exist. */
    fun exists(): Boolean = keyStore.containsAlias(ALIAS) && wrappedFile.isFile

    /**
     * Creates the wrapping key, replacing any existing one. Requires a
     * secure lock screen. Returns whether it is StrongBox-backed.
     *
     * @param strict accept only the currently enrolled strong biometrics;
     *   enrolling a new one then invalidates the key (unlock with the
     *   recovery key instead).
     */
    fun createWrappingKey(strict: Boolean): Boolean {
        keyStore.deleteEntry(ALIAS)
        val authenticators = if (strict) {
            KeyProperties.AUTH_BIOMETRIC_STRONG
        } else {
            KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL
        }
        fun spec(strongBox: Boolean) =
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(KEY_BITS)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(true)
                // Timeout 0: authentication is required for every single use.
                .setUserAuthenticationParameters(0, authenticators)
                .setInvalidatedByBiometricEnrollment(strict)
                .setUnlockedDeviceRequired(true)
                .setIsStrongBoxBacked(strongBox)
                .build()
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        return try {
            generator.init(spec(strongBox = true))
            generator.generateKey()
            true
        } catch (_: StrongBoxUnavailableException) {
            generator.init(spec(strongBox = false))
            generator.generateKey()
            false
        }
    }

    /** A cipher to authenticate (BiometricPrompt) and then pass to [store]. */
    fun encryptCipher(): Cipher = Cipher.getInstance(TRANSFORMATION).apply {
        init(Cipher.ENCRYPT_MODE, wrappingKey())
    }

    /**
     * A cipher to authenticate and then pass to [unwrap].
     *
     * @throws android.security.keystore.KeyPermanentlyInvalidatedException
     *   if strict mode was on and the enrolled biometrics changed.
     */
    fun decryptCipher(): Cipher {
        val wrapped = readWrapped()
        return Cipher.getInstance(TRANSFORMATION).apply {
            init(Cipher.DECRYPT_MODE, wrappingKey(), GCMParameterSpec(TAG_BITS, wrapped.iv))
        }
    }

    /** Wraps and saves the master key with an authenticated [encryptCipher]. */
    fun store(authenticated: Cipher, masterKey: ByteArray) {
        val wrapped = WrappedMasterKey(authenticated.iv, authenticated.doFinal(masterKey))
        wrappedFile.parentFile?.mkdirs()
        val tmp = File(wrappedFile.path + ".tmp")
        tmp.writeBytes(wrapped.toBytes())
        if (!tmp.renameTo(wrappedFile)) {
            tmp.delete()
            error("could not save wrapped key")
        }
    }

    /**
     * Unwraps the master key with an authenticated [decryptCipher]. The
     * caller must wipe the returned array (`fill(0)`) as soon as the core
     * has it.
     */
    fun unwrap(authenticated: Cipher): ByteArray = authenticated.doFinal(readWrapped().ciphertext)

    /** Deletes the wrapping key and the wrapped master key. */
    fun destroy() {
        keyStore.deleteEntry(ALIAS)
        wrappedFile.delete()
    }

    private fun wrappingKey(): SecretKey =
        keyStore.getKey(ALIAS, null) as? SecretKey ?: error("wrapping key missing")

    private fun readWrapped(): WrappedMasterKey =
        WrappedMasterKey.fromBytes(wrappedFile.readBytes()) ?: error("wrapped key malformed")

    companion object {
        /** Keystore alias of the wrapping key. */
        const val ALIAS = "terpoir.master-wrap"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val KEY_BITS = 256
        private const val TAG_BITS = 128
    }
}
